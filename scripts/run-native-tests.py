"""Run every Rust unit and real-engine regression test; retain full evidence."""
import subprocess,sys,argparse
from pathlib import Path
root=Path(__file__).resolve().parents[1]
report=root/'.test-engines/native-tests.log';report.parent.mkdir(exist_ok=True)
parser=argparse.ArgumentParser();parser.add_argument('--headless',action='store_true');args=parser.parse_args()
command=['cargo','test','--locked','--manifest-path','src-tauri/Cargo.toml']
if args.headless:command+=['--no-default-features','--lib']
command+=['--','--include-ignored','--test-threads=1','--nocapture']
with report.open('w',encoding='utf-8') as log:
    proc=subprocess.Popen(command,cwd=root,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,encoding='utf-8',errors='replace')
    for line in proc.stdout:
        sys.stdout.write(line);log.write(line)
    status=proc.wait()
sys.exit(status)
