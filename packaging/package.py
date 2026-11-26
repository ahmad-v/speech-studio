#!/usr/bin/env python3
"""Experimental packaging only. Runs on the native build host, never inside the studio."""
import argparse, hashlib, json, os, pathlib, platform, plistlib, shutil, subprocess, tempfile, zipfile
R=pathlib.Path(__file__).resolve().parents[1]
CFG=json.loads((R/'packaging/config.json').read_text())
TARGETS={x['target']:x for x in CFG['targets']}

def call(*args): subprocess.run([str(a) for a in args],check=True)
def need(tool):
    p=shutil.which(tool)
    if not p: raise SystemExit(f'Missing reviewed build tool: {tool}')
    return p

def validate_host(target):
    arch=platform.machine().lower(); osname=platform.system()
    expected_arch='arm64' if target.startswith('aarch64') else 'x64'
    actual='arm64' if arch in ('arm64','aarch64') else 'x64' if arch in ('amd64','x86_64') else arch
    expected_os='Windows' if 'windows' in target else 'Darwin' if 'apple' in target else 'Linux'
    if (actual,osname)!=(expected_arch,expected_os): raise SystemExit(f'Native host required: {expected_os}/{expected_arch}; got {osname}/{arch}')

def check_binary(path,target):
    with path.open('rb') as f:
        header=f.read(64)
        arm=target.startswith('aarch64')
        if 'windows' in target:
            if header[:2]!=b'MZ': raise SystemExit('Not PE')
            f.seek(int.from_bytes(header[60:64],'little')); pe=f.read(6)
            if pe[:4]!=b'PE\0\0' or int.from_bytes(pe[4:6],'little')!=(0xaa64 if arm else 0x8664): raise SystemExit('Wrong PE architecture')
        elif 'linux' in target:
            if header[:6]!=b'\x7fELF\x02\x01' or int.from_bytes(header[18:20],'little')!=(183 if arm else 62): raise SystemExit('Wrong ELF architecture')
        else:
            if header[:4]!=b'\xcf\xfa\xed\xfe' or int.from_bytes(header[4:8],'little')!=0x0100000c: raise SystemExit('Not native ARM64 Mach-O')

def payload(dest,binpath):
    dest.mkdir(parents=True,exist_ok=True);shutil.copy2(binpath,dest/binpath.name)
    for p in ('README.md','LICENSE','LICENSE-MIT','LICENSE-APACHE','THIRD_PARTY_NOTICES.md','CITATION.cff'):
        shutil.copy2(R/p,dest/p)
    shutil.copy2(R/'docs/MANUAL.md',dest/'MANUAL.md')
    shutil.copy2(R/'docs/capabilities.json',dest/'capabilities.json')
    # No private engines are enabled in baseline builds, including Speech optional ASR.
    (dest/'PAYLOAD_STATUS.txt').write_text('UNVERIFIED DEVELOPMENT ARTIFACT. No private model or engine pack included. Not for release.\n')

def linux_tree(root,binpath):
    n=CFG['name'];d=root/'usr/lib'/n;payload(d,binpath)
    (root/'usr/bin').mkdir(parents=True,exist_ok=True);(root/'usr/bin'/n).symlink_to(f'../lib/{n}/{n}')
    for relative,src in [('usr/share/applications',CFG['app_id']+'.desktop'),('usr/share/metainfo',CFG['app_id']+'.metainfo.xml'),('usr/share/icons/hicolor/scalable/apps',CFG['app_id']+'.svg')]:
        dest=root/relative;dest.mkdir(parents=True,exist_ok=True);shutil.copy2(R/'packaging/linux'/src,dest/src)

def build(fmt,target,binpath,out):
    n,title,ver=CFG['name'],CFG['title'],CFG['package_version'];arm=target.startswith('aarch64');stem=f'{n}-{ver}-{target}'
    with tempfile.TemporaryDirectory(prefix='studio-package-') as tmp:
        t=pathlib.Path(tmp);tree=t/'payload';payload(tree,binpath)
        if fmt=='zip':
            dest=out/(stem+'-portable.zip')
            with zipfile.ZipFile(dest,'x',zipfile.ZIP_DEFLATED) as z:
                for p in sorted(tree.rglob('*')):
                    if p.is_file():z.write(p,arcname=f'{n}/{p.relative_to(tree).as_posix()}')
        elif fmt=='deb':
            root=t/'deb';linux_tree(root,binpath);control=root/'DEBIAN';control.mkdir()
            (control/'control').write_text(f'Package: {n}\nVersion: {ver}\nArchitecture: {"arm64" if arm else "amd64"}\nMaintainer: {CFG["maintainer"]}\nSection: utils\nPriority: optional\nDepends: libc6, libgcc-s1, libxkbcommon0, libwayland-client0, libx11-6, libfontconfig1\nDescription: {title} experimental development package\n')
            call(need('dpkg-deb'),'--root-owner-group','--build',root,out/(stem+'.deb'))
        elif fmt=='rpm':
            root=t/'rpm';linux_tree(root,binpath);top=t/'rpmbuild'
            for sub in ('BUILD','BUILDROOT','RPMS','SOURCES','SPECS','SRPMS'):(top/sub).mkdir(parents=True)
            source=top/'SPECS'/f'{n}.spec';source.write_text(f'Name: {n}\nVersion: {ver.split("~")[0]}\nRelease: 0.1.alpha1\nSummary: {title} development snapshot\nLicense: MIT OR Apache-2.0\nAutoReqProv: yes\n%description\nExperimental, unverified.\n%install\nmkdir -p %{{buildroot}}\ncp -a "{root}/." %{{buildroot}}/\n%files\n/usr/lib/{n}\n/usr/bin/{n}\n/usr/share/applications/{CFG["app_id"]}.desktop\n/usr/share/metainfo/{CFG["app_id"]}.metainfo.xml\n/usr/share/icons/hicolor/scalable/apps/{CFG["app_id"]}.svg\n')
            call(need('rpmbuild'),'--define',f'_topdir {top}','--target','aarch64' if arm else 'x86_64','-bb',source)
            rpms=list((top/'RPMS').rglob('*.rpm'))
            if len(rpms)!=1:raise SystemExit('Unexpected RPM outputs')
            shutil.copy2(rpms[0],out/rpms[0].name)
        elif fmt=='dmg':
            app=t/(title+'.app');mac=app/'Contents/MacOS';mac.mkdir(parents=True);shutil.copy2(binpath,mac/n);resources=app/'Contents/Resources';payload(resources,binpath);(resources/binpath.name).unlink()
            with (app/'Contents/Info.plist').open('wb') as f:plistlib.dump({'CFBundleIdentifier':CFG['app_id'],'CFBundleName':title,'CFBundleExecutable':n,'CFBundlePackageType':'APPL','CFBundleVersion':'1','CFBundleShortVersionString':'0.1.0','LSArchitecturePriority':['arm64'],'NSHighResolutionCapable':True},f)
            call(need('codesign'),'--force','--sign','-',app);call('codesign','--verify','--strict',app)
            image=t/'image';image.mkdir();shutil.move(app,image/app.name);(image/'Applications').symlink_to('/Applications');call(need('hdiutil'),'create','-volname',title,'-srcfolder',image,'-format','UDZO',out/(stem+'.dmg'))
        elif fmt=='msi':
            # WiX v4+ must be installed and reviewed separately. Stable product family GUID is provisional.
            source=R/'packaging/windows/product.wxs';call(need('wix'),'build','-arch','arm64' if arm else 'x64','-d',f'Payload={tree}','-d',f'Executable={binpath.name}','-o',out/(stem+'.msi'),source)
        elif fmt=='exe':
            raise SystemExit('Blocked: verify native offline WiX Burn bootstrapper for this architecture and same MSI payload before enabling. See windows/bundle.wxs.template.')
        elif fmt=='appimage':
            # appimagetool needs a matching reviewed runtime and a SHA-pinned tool; not acquired here.
            tool=os.environ.get('APPIMAGETOOL');digest=os.environ.get('APPIMAGETOOL_SHA256')
            if not tool or not digest:raise SystemExit('Set reviewed appimagetool absolute path and SHA256 first')
            p=pathlib.Path(tool)
            if not p.is_absolute() or hashlib.sha256(p.read_bytes()).hexdigest()!=digest:raise SystemExit('Invalid appimagetool path or digest')
            appdir=t/'AppDir';linux_tree(appdir,binpath);(appdir/'AppRun').symlink_to(f'usr/lib/{n}/{n}')
            shutil.copy2(R/'packaging/linux'/(CFG['app_id']+'.desktop'),appdir/(CFG['app_id']+'.desktop'));shutil.copy2(R/'packaging/linux'/(CFG['app_id']+'.svg'),appdir/(CFG['app_id']+'.svg'))
            call(p,'--appimage-extract-and-run',appdir,out/(stem+'.AppImage'))
        elif fmt=='flatpak':
            raise SystemExit('Blocked: generate reviewed Cargo vendor source and pinned runtime commits, then use flatpak-builder with packaging/flatpak/manifest.yml.template.')
        else:raise SystemExit('Unknown format')

def main():
    a=argparse.ArgumentParser();a.add_argument('--target',choices=TARGETS,required=True);a.add_argument('--format',required=True);a.add_argument('--binary',type=pathlib.Path);args=a.parse_args();target=args.target;validate_host(target)
    if not (R/'Cargo.lock').is_file():raise SystemExit('Reviewed Cargo.lock required')
    if not CFG.get('approved_identity') or not CFG.get('maintainer'):raise SystemExit('Set reviewed package identity and maintainer in config before packaging (all IDs provisional)')
    ext='.exe' if 'windows' in target else '';binary=args.binary or R/'target'/target/'release'/(CFG['name']+ext)
    if not binary.is_file():raise SystemExit('Build locked payload first')
    check_binary(binary,target);formats=TARGETS[target]['formats'];requested=TARGETS[target]['baseline'] if args.format=='baseline' else [args.format]
    if any(f not in formats for f in requested):raise SystemExit('Format not valid for target')
    out=R/'dist'/target;out.mkdir(parents=True,exist_ok=False)
    for fmt in requested:build(fmt,target,binary,out)
    with (out/'SHA256SUMS').open('w') as f:
        for p in sorted(out.iterdir()):
            if p.is_file() and p.name!='SHA256SUMS':f.write(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n')
if __name__=='__main__':main()
