//! Functional tests of the production runner, with real native engines.
//! Ignored in the fast suite, required in CI via `cargo test regression_ -- --ignored`.
use super::*;
use serde_json::Value;
use std::sync::atomic::Ordering;

struct Lab {
    dir: tempfile::TempDir,
    db: PathBuf,
}
impl Lab {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("tool-kit régression espaces ")
            .tempdir()
            .unwrap();
        let db = dir.path().join("tasks.sqlite3");
        crate::store::init(&db).unwrap();
        Self { dir, db }
    }
    fn file(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }
    fn request(&self, op: &str, inputs: &[&Path], format: &str) -> TaskRequest {
        TaskRequest {
            operation: op.into(),
            inputs: inputs.iter().map(|p| p.to_str().unwrap().into()).collect(),
            destination: self.dir.path().to_str().unwrap().into(),
            format: format.into(),
            pages: "1".into(),
            width: 32,
            quality: 23,
            track: 0,
        }
    }
    fn run(&self, r: &TaskRequest) -> Result<PathBuf, String> {
        self.run_cancel(r, &Arc::new(AtomicBool::new(false)))
    }
    fn run_cancel(&self, r: &TaskRequest, cancel: &Arc<AtomicBool>) -> Result<PathBuf, String> {
        let name = engine_for(&r.operation)?;
        let before: Vec<_> = r
            .inputs
            .iter()
            .filter_map(|p| fs::read(p).ok().map(|bytes| (p.clone(), bytes)))
            .collect();
        let result = run_with_binaries(
            &self.db,
            r,
            cancel,
            &EngineBinaries {
                main: binary(name),
                probe: if name == "ffmpeg" {
                    Some(binary("ffprobe"))
                } else {
                    None
                },
            },
        );
        for (path, bytes) in before {
            assert_eq!(fs::read(&path).unwrap(), bytes, "source modified: {path}");
        }
        result
    }
    fn clean(&self) {
        assert!(
            !fs::read_dir(self.dir.path()).unwrap().any(|e| e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".toolkit-")),
            "temporary output leaked"
        );
        let count: i64 = crate::store::connect(&self.db)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM temporary_outputs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "temporary registry leaked");
    }
}
fn binary(engine: &str) -> PathBuf {
    let name = match engine {
        "imagemagick" => "TOOLKIT_MAGICK",
        "ffmpeg" => "TOOLKIT_FFMPEG",
        "ffprobe" => "TOOLKIT_FFPROBE",
        _ => "TOOLKIT_QPDF",
    };
    let p = PathBuf::from(
        std::env::var_os(name)
            .unwrap_or_else(|| panic!("{name} is mandatory for real-engine regression tests")),
    );
    assert!(
        p.is_absolute() && p.is_file(),
        "{name}: missing executable {}",
        p.display()
    );
    p
}
fn checked(bin: &Path, args: &[&str]) -> Vec<u8> {
    let out = Command::new(bin).args(args).output().unwrap();
    assert!(
        out.status.success(),
        "{} {:?}: {}",
        bin.display(),
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}
fn text(bin: &Path, args: &[&str]) -> String {
    String::from_utf8(checked(bin, args)).unwrap()
}
fn probe(p: &Path) -> Value {
    serde_json::from_slice(&checked(
        &binary("ffprobe"),
        &[
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
            p.to_str().unwrap(),
        ],
    ))
    .unwrap()
}
fn wav(lab: &Lab, name: &str, frequency: u32) -> PathBuf {
    let p = lab.file(name);
    checked(
        &binary("ffmpeg"),
        &[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency={frequency}:sample_rate=44100:duration=0.5"),
            "-c:a",
            "pcm_s16le",
            p.to_str().unwrap(),
        ],
    );
    p
}
fn pcm(p: &Path) -> Vec<u8> {
    checked(
        &binary("ffmpeg"),
        &[
            "-v",
            "error",
            "-i",
            p.to_str().unwrap(),
            "-vn",
            "-ac",
            "1",
            "-ar",
            "44100",
            "-f",
            "s16le",
            "pipe:1",
        ],
    )
}
fn frequency(p: &Path) -> f64 {
    let data = pcm(p);
    let samples: Vec<i16> = data
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect();
    let crossings = samples.windows(2).filter(|s| s[0] <= 0 && s[1] > 0).count();
    crossings as f64 / (samples.len() as f64 / 44100.0)
}
fn image(lab: &Lab, name: &str, size: &str, color: &str) -> PathBuf {
    let p = lab.file(name);
    checked(
        &binary("imagemagick"),
        &["-size", size, &format!("xc:{color}"), p.to_str().unwrap()],
    );
    p
}
fn image_info(p: &Path) -> String {
    text(
        &binary("imagemagick"),
        &["identify", "-format", "%m %w %h", p.to_str().unwrap()],
    )
}
fn rgba(p: &Path) -> Vec<u8> {
    checked(
        &binary("imagemagick"),
        &[p.to_str().unwrap(), "-depth", "8", "rgba:-"],
    )
}
fn pdf(lab: &Lab, name: &str, labels: &[&str]) -> PathBuf {
    let mut objects = vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(), Vec::new()];
    let mut children = Vec::new();
    for label in labels {
        let page = objects.len() + 1;
        let stream = page + 1;
        children.push(format!("{page} 0 R"));
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Contents {stream} 0 R >>").into_bytes());
        let content = format!("BT /F1 14 Tf 20 100 Td ({label}) Tj ET\n");
        objects.push(
            format!(
                "<< /Length {} >>\nstream\n{}endstream",
                content.len(),
                content
            )
            .into_bytes(),
        );
    }
    objects[1] = format!(
        "<< /Type /Pages /Count {} /Kids [{}] >>",
        labels.len(),
        children.join(" ")
    )
    .into_bytes();
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        bytes.extend_from_slice(obj);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    let p = lab.file(name);
    fs::write(&p, bytes).unwrap();
    p
}
fn pdf_labels(path: &Path) -> Vec<String> {
    let q = binary("qpdf");
    let pages = text(&q, &["--show-pages", path.to_str().unwrap()]);
    let mut labels = Vec::new();
    for line in pages.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() == 3 && fields[1] == "0" && fields[2] == "R" {
            let stream = text(
                &q,
                &[
                    &format!("--show-object={}", fields[0]),
                    "--filtered-stream-data",
                    path.to_str().unwrap(),
                ],
            );
            labels.push(
                stream
                    .split('(')
                    .nth(1)
                    .unwrap()
                    .split(')')
                    .next()
                    .unwrap()
                    .into(),
            );
        }
    }
    labels
}

#[test]
#[ignore = "real native engines"]
fn regression_pdf_merge_preserves_page_order_and_sources() {
    let lab = Lab::new();
    let a = pdf(&lab, "premier é.pdf", &["FIRST", "SECOND"]);
    let b = pdf(&lab, "deuxième.pdf", &["THIRD"]);
    let original = fs::read(&a).unwrap();
    let out = lab
        .run(&lab.request("pdf.merge", &[&b, &a], "pdf"))
        .unwrap();
    assert_eq!(pdf_labels(&out), ["THIRD", "FIRST", "SECOND"]);
    assert_eq!(fs::read(a).unwrap(), original);
    lab.clean();
}
#[test]
#[ignore = "real native engines"]
fn regression_pdf_extract_selects_requested_pages() {
    let lab = Lab::new();
    let src = pdf(&lab, "pages.pdf", &["ONE", "TWO", "THREE", "FOUR"]);
    let mut req = lab.request("pdf.extract", &[&src], "pdf");
    req.pages = "1-2,4".into();
    let out = lab.run(&req).unwrap();
    assert_eq!(pdf_labels(&out), ["ONE", "TWO", "FOUR"]);
    lab.clean();
}
#[test]
#[ignore = "real native engines"]
fn regression_pdf_rejects_invalid_ranges_encrypted_and_corrupt_inputs() {
    let lab = Lab::new();
    let src = pdf(&lab, "valid.pdf", &["ONE"]);
    let mut req = lab.request("pdf.extract", &[&src], "pdf");
    req.pages = "2".into();
    assert!(lab.run(&req).unwrap_err().contains("pages"));
    lab.clean();
    let protected = lab.file("protected.pdf");
    checked(
        &binary("qpdf"),
        &[
            "--encrypt",
            "secret",
            "owner-secret",
            "256",
            "--",
            src.to_str().unwrap(),
            protected.to_str().unwrap(),
        ],
    );
    req.inputs = vec![protected.to_str().unwrap().into()];
    req.pages = "1".into();
    assert!(lab.run(&req).unwrap_err().contains("protégé"));
    lab.clean();
    fs::write(&protected, b"broken PDF").unwrap();
    assert!(lab.run(&req).is_err());
    lab.clean();
}
#[test]
#[ignore = "real native engines"]
fn regression_audio_all_formats_preserve_signal_and_lossless_samples() {
    let lab = Lab::new();
    let src = wav(&lab, "son accentué.wav", 440);
    let original = fs::read(&src).unwrap();
    let raw = pcm(&src);
    for (format, codec) in [("mp3", "mp3"), ("wav", "pcm_s16le"), ("flac", "flac")] {
        let out = lab
            .run(&lab.request("audio.convert", &[&src], format))
            .unwrap();
        let info = probe(&out);
        assert_eq!(info["streams"][0]["codec_name"], codec);
        let duration = info["format"]["duration"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap();
        assert!((duration - 0.5).abs() < 0.1);
        assert!(
            (frequency(&out) - 440.0).abs() < 8.0,
            "audio signal changed"
        );
        if format != "mp3" {
            assert_eq!(pcm(&out), raw);
        }
        lab.clean();
    }
    assert_eq!(fs::read(src).unwrap(), original);
}
fn video(lab: &Lab, audio: bool) -> PathBuf {
    let p = lab.file(if audio {
        "vidéo multi pistes.mkv"
    } else {
        "silencieuse.mkv"
    });
    let mut c = Command::new(binary("ffmpeg"));
    c.args([
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=80x64:rate=10:duration=0.5",
    ]);
    if audio {
        c.args([
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.5",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:duration=0.5",
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-map",
            "2:a",
            "-c:a",
            "pcm_s16le",
        ]);
    }
    c.args(["-c:v", "ffv1"]).arg(&p);
    let o = c.output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    p
}
#[test]
#[ignore = "real native engines"]
fn regression_video_mp4_has_decodable_h264_aac_and_expected_dimensions() {
    let lab = Lab::new();
    let src = video(&lab, true);
    let out = lab
        .run(&lab.request("video.convert", &[&src], "mp4"))
        .unwrap();
    let p = probe(&out);
    let streams = p["streams"].as_array().unwrap();
    assert_eq!(streams.len(), 2);
    assert_eq!(streams[0]["codec_name"], "h264");
    assert_eq!(streams[1]["codec_name"], "aac");
    assert_eq!(streams[0]["width"], 80);
    assert_eq!(streams[0]["height"], 64);
    let decoded = checked(
        &binary("ffmpeg"),
        &[
            "-v",
            "error",
            "-i",
            out.to_str().unwrap(),
            "-an",
            "-pix_fmt",
            "rgb24",
            "-f",
            "rawvideo",
            "pipe:1",
        ],
    );
    assert_eq!(decoded.len(), 80 * 64 * 3 * 5);
    assert!(decoded.iter().any(|x| *x > 128));
    lab.clean();
}
#[test]
#[ignore = "real native engines"]
fn regression_silent_video_converts_and_audio_extraction_selects_track() {
    let lab = Lab::new();
    let silent = video(&lab, false);
    let out = lab
        .run(&lab.request("video.convert", &[&silent], "mp4"))
        .unwrap();
    assert_eq!(probe(&out)["streams"].as_array().unwrap().len(), 1);
    let src = video(&lab, true);
    for format in ["mp3", "wav", "flac"] {
        let mut req = lab.request("video.audio", &[&src], format);
        req.track = 1;
        let audio = lab.run(&req).unwrap();
        assert!((frequency(&audio) - 880.0).abs() < 10.0);
    }
    let mut missing = lab.request("video.audio", &[&src], "wav");
    missing.track = 9;
    assert!(lab.run(&missing).is_err());
    assert!(lab
        .run(&lab.request("video.audio", &[&silent], "mp3"))
        .is_err());
    lab.clean();
}
#[test]
#[ignore = "real native engines"]
fn regression_image_converts_all_format_pairs_and_flattens_alpha_for_jpeg() {
    let lab = Lab::new();
    for input_format in ["jpeg", "png", "webp"] {
        let src = image(&lab, &format!("image é.{input_format}"), "80x40", "red");
        for output_format in ["jpeg", "png", "webp"] {
            let out = lab
                .run(&lab.request("image.convert", &[&src], output_format))
                .unwrap();
            let info = image_info(&out);
            assert!(info.ends_with("80 40"), "{info}");
            let pixels = rgba(&out);
            assert_eq!(pixels.len(), 80 * 40 * 4);
            assert!(pixels[0] > 240 && pixels[1] < 15 && pixels[2] < 15);
            lab.clean();
        }
    }
    let src = image(&lab, "transparent.png", "10x10", "none");
    let out = lab
        .run(&lab.request("image.convert", &[&src], "jpeg"))
        .unwrap();
    let pixels = rgba(&out);
    assert_eq!(&pixels[..4], &[255, 255, 255, 255]);
    lab.clean();
}
#[test]
#[ignore = "real native engines"]
fn regression_resize_preserves_ratio_and_never_enlarges() {
    let lab = Lab::new();
    let src = image(&lab, "large.png", "80x40", "blue");
    let mut req = lab.request("image.resize", &[&src], "png");
    req.width = 32;
    let out = lab.run(&req).unwrap();
    assert_eq!(image_info(&out), "PNG 32 16");
    req.width = 160;
    let out = lab.run(&req).unwrap();
    assert_eq!(image_info(&out), "PNG 80 40");
    lab.clean();
}
#[test]
#[ignore = "real native engines"]
fn regression_invalid_files_never_publish_partial_results() {
    let lab = Lab::new();
    let src = lab.file("invalide fichier.bin");
    fs::write(&src, b"not a valid file").unwrap();
    for (operation, format) in [
        ("pdf.extract", "pdf"),
        ("audio.convert", "mp3"),
        ("video.convert", "mp4"),
        ("video.audio", "wav"),
        ("image.convert", "png"),
        ("image.resize", "png"),
    ] {
        assert!(
            lab.run(&lab.request(operation, &[&src], format)).is_err(),
            "{operation}"
        );
        lab.clean();
    }
    assert_eq!(fs::read(src).unwrap(), b"not a valid file");
}
#[test]
#[ignore = "real native engines"]
fn regression_collision_keeps_existing_results_and_unavailable_destination_fails() {
    let lab = Lab::new();
    let src = wav(&lab, "collision.wav", 440);
    let req = lab.request("audio.convert", &[&src], "flac");
    let first = lab.run(&req).unwrap();
    let bytes = fs::read(&first).unwrap();
    let second = lab.run(&req).unwrap();
    assert_ne!(first, second);
    assert_eq!(fs::read(&first).unwrap(), bytes);
    let mut bad = req;
    bad.destination = lab.file("nonexistent").to_str().unwrap().into();
    assert!(lab.run(&bad).is_err());
    lab.clean();
}
#[test]
#[ignore = "real native engines"]
fn regression_cancelled_before_execution_creates_nothing() {
    let lab = Lab::new();
    let src = wav(&lab, "cancel.wav", 440);
    let req = lab.request("audio.convert", &[&src], "flac");
    let flag = Arc::new(AtomicBool::new(true));
    assert_eq!(lab.run_cancel(&req, &flag).unwrap_err(), "Annulé");
    lab.clean();
    assert!(src.is_file());
}
#[test]
#[ignore = "real native engines"]
fn regression_running_process_can_be_cancelled_promptly() {
    let lab = Lab::new();
    let flag = Arc::new(AtomicBool::new(false));
    let other = flag.clone();
    let start = std::time::Instant::now();
    let mut c = Command::new(binary("ffmpeg"));
    c.args([
        "-v",
        "error",
        "-re",
        "-f",
        "lavfi",
        "-i",
        "sine=duration=30",
        "-f",
        "null",
        "-",
    ]);
    let handle = thread::spawn(move || execute(c, &other, false));
    thread::sleep(Duration::from_millis(400));
    flag.store(true, Ordering::SeqCst);
    assert_eq!(handle.join().unwrap().unwrap_err(), "Annulé");
    assert!(start.elapsed() < Duration::from_secs(5));
    lab.clean();
}
#[test]
#[ignore = "real native engines"]
fn regression_animated_webp_is_refused_without_partial_output() {
    let lab = Lab::new();
    let src = lab.file("animation.webp");
    checked(
        &binary("imagemagick"),
        &[
            "-delay",
            "10",
            "-size",
            "20x20",
            "xc:red",
            "xc:blue",
            "-loop",
            "0",
            src.to_str().unwrap(),
        ],
    );
    assert!(lab
        .run(&lab.request("image.convert", &[&src], "png"))
        .unwrap_err()
        .contains("animées"));
    lab.clean();
}

#[test]
#[ignore = "real native engines"]
fn regression_exif_orientation_rotates_pixels_and_dimensions() {
    let lab = Lab::new();
    let src = lab.file("orientation.jpg");
    fs::write(&src, include_bytes!("fixtures/oriented.jpg")).unwrap();
    let out = lab
        .run(&lab.request("image.convert", &[&src], "png"))
        .unwrap();
    assert_eq!(image_info(&out), "PNG 8 12");
    let pixels = rgba(&out);
    assert!(
        pixels[0] > 240 && pixels[2] < 15,
        "top of rotated image must be red"
    );
    assert!(
        pixels[pixels.len() - 2] > 240 && pixels[pixels.len() - 4] < 15,
        "bottom must be blue"
    );
    lab.clean();
}

#[test]
#[ignore = "real native engines"]
fn regression_odd_video_dimensions_are_padded_for_h264() {
    let lab = Lab::new();
    let src = lab.file("odd.mkv");
    checked(
        &binary("ffmpeg"),
        &[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=81x65:rate=5:duration=0.4",
            "-c:v",
            "ffv1",
            src.to_str().unwrap(),
        ],
    );
    let out = lab
        .run(&lab.request("video.convert", &[&src], "mp4"))
        .unwrap();
    let p = probe(&out);
    assert_eq!(p["streams"][0]["width"], 82);
    assert_eq!(p["streams"][0]["height"], 66);
    assert_eq!(p["streams"][0]["codec_name"], "h264");
    lab.clean();
}

#[cfg(unix)]
#[test]
#[ignore = "real native engines"]
fn regression_readonly_destination_preserves_source_and_creates_no_partial_output() {
    use std::os::unix::fs::PermissionsExt;
    let lab = Lab::new();
    let src = wav(&lab, "source.wav", 440);
    let original = fs::read(&src).unwrap();
    let dest = lab.file("readonly");
    fs::create_dir(&dest).unwrap();
    fs::set_permissions(&dest, fs::Permissions::from_mode(0o555)).unwrap();
    let mut req = lab.request("audio.convert", &[&src], "flac");
    req.destination = dest.to_str().unwrap().into();
    let result = lab.run(&req);
    fs::set_permissions(&dest, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read(src).unwrap(), original);
    assert_eq!(fs::read_dir(dest).unwrap().count(), 0);
    lab.clean();
}
