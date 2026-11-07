"""Resolve CI-only native engines. Never modifies the application catalogue."""
import hashlib,json,os,platform,shutil,subprocess,urllib.request
from pathlib import Path
root=Path(__file__).resolve().parents[1]
work=root/'.test-engines';work.mkdir(exist_ok=True)

def resolve(name):
    path=shutil.which(name)
    if path:return str(Path(path).resolve())
    if os.name=='nt':
        patterns={'qpdf':['qpdf*/bin/qpdf.exe','qpdf*/**/qpdf.exe'], 'magick':['ImageMagick*/magick.exe'], 'ffmpeg':['ffmpeg*/bin/ffmpeg.exe'], 'ffprobe':['ffmpeg*/bin/ffprobe.exe']}
        for base in [Path('C:/Program Files'),Path('C:/tools'),Path('C:/ProgramData/chocolatey/lib')]:
            for pattern in patterns[name]:
                match=next(base.glob(pattern),None)
                if match:return str(match.resolve())
    raise RuntimeError(f'Missing real engine: {name}; regression suite cannot run')

if os.name!='nt':
    url='https://github.com/ImageMagick/ImageMagick/releases/download/7.1.2-35/ImageMagick-7.1.2-35-gcc-x86_64.AppImage'
    digest='f111044ed06b68cfb478df724c5a32d35145afa21073242f888d234a2f8cc002'
    appimage=work/'imagemagick.AppImage'
    with urllib.request.urlopen(url,timeout=120) as response,appimage.open('wb') as out:shutil.copyfileobj(response,out)
    if hashlib.sha256(appimage.read_bytes()).hexdigest()!=digest:raise RuntimeError('ImageMagick test artifact checksum mismatch')
    appimage.chmod(0o755)
    subprocess.run([str(appimage),'--appimage-extract'],cwd=work,check=True,stdout=subprocess.DEVNULL)
    magick=str(work/'squashfs-root/AppRun')
else:magick=resolve('magick')
paths={'TOOLKIT_QPDF':resolve('qpdf'),'TOOLKIT_FFMPEG':resolve('ffmpeg'),'TOOLKIT_FFPROBE':resolve('ffprobe'),'TOOLKIT_MAGICK':magick}
versions={}
for key,exe in paths.items():
    out=subprocess.run([exe,'--version' if key=='TOOLKIT_QPDF' else '-version'],check=True,capture_output=True,text=True)
    versions[key]={'path':exe,'version':out.stdout.splitlines()[0]}
print(json.dumps(versions,ensure_ascii=False,indent=2))
(work/'versions.json').write_text(json.dumps({'os':platform.platform(),'engines':versions},ensure_ascii=False,indent=2),encoding='utf-8')
if 'GITHUB_ENV' in os.environ:
    with open(os.environ['GITHUB_ENV'],'a',encoding='utf-8') as f:
        for key,path in paths.items():f.write(f'{key}={path}\n')
