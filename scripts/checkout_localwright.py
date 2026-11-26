import json, pathlib, re, subprocess
root=pathlib.Path(__file__).resolve().parents[1]
c=json.loads((root/'integration/localwright.json').read_text())
repo,rev=c.get('repository'),c.get('revision')
if not isinstance(repo,str) or not re.fullmatch(r'https://github\.com/[A-Za-z0-9_.-]+/localwright(?:\.git)?',repo):raise SystemExit('Set approved canonical Localwright URL first')
if not isinstance(rev,str) or not re.fullmatch(r'[0-9a-f]{40}',rev):raise SystemExit('Set real immutable Localwright SHA first')
dest=root.parent/'localwright'
if dest.exists():raise SystemExit('Refusing to replace existing sibling directory')
subprocess.run(['git','init',str(dest)],check=True)
subprocess.run(['git','-C',str(dest),'fetch','--depth=1',repo,rev],check=True)
subprocess.run(['git','-C',str(dest),'checkout','--detach','FETCH_HEAD'],check=True)
actual=subprocess.check_output(['git','-C',str(dest),'rev-parse','HEAD'],text=True).strip()
if actual!=rev:raise SystemExit('Unexpected revision')
