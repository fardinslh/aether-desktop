#!/usr/bin/env python3
"""Fetch pinned official runtimes; verify release SHA-256 before safe extraction."""
import argparse, hashlib, json, pathlib, shutil, tarfile, tempfile, urllib.request, zipfile
ROOT = pathlib.Path(__file__).resolve().parents[1]
VERSIONS = {'CluvexStudio/Aether': 'v2.3.0', 'SagerNet/sing-box': 'v1.14.3', 'heiher/sockstun':'8.0'}

def fetch(url):
    return urllib.request.urlopen(urllib.request.Request(url, headers={'User-Agent':'AetherDesktop-build'}), timeout=120).read()

def archive(repo, name):
    release=json.loads(fetch(f'https://api.github.com/repos/{repo}/releases/tags/{VERSIONS[repo]}'))
    asset=next(a for a in release['assets'] if a['name']==name)
    expected=asset.get('digest','').removeprefix('sha256:')
    if len(expected)!=64:
        checksum=next(a for a in release['assets'] if a['name']==name+'.sha256')
        expected=fetch(checksum['browser_download_url']).decode().split()[0]
    data=fetch(asset['browser_download_url'])
    if hashlib.sha256(data).hexdigest()!=expected: raise RuntimeError(f'Checksum mismatch: {name}')
    return data,expected

def extract(data, name, destination):
    with tempfile.TemporaryDirectory() as tmp:
        source=pathlib.Path(tmp)/name; source.write_bytes(data)
        if name.endswith('.zip'):
            with zipfile.ZipFile(source) as z:
                for i in z.infolist():
                    p=pathlib.PurePosixPath(i.filename)
                    if p.is_absolute() or '..' in p.parts: raise RuntimeError('Unsafe archive path')
                z.extractall(tmp+'/unpacked')
        else:
            with tarfile.open(source) as t:
                for i in t.getmembers():
                    p=pathlib.PurePosixPath(i.name)
                    if p.is_absolute() or '..' in p.parts or not (i.isfile() or i.isdir()): raise RuntimeError('Unsafe tar member')
                t.extractall(tmp+'/unpacked')
        for f in pathlib.Path(tmp+'/unpacked').rglob('*'):
            if not f.is_file(): continue
            # Aether's pt directory is part of the runtime, never discard it.
            parts=f.relative_to(tmp+'/unpacked').parts
            rel=pathlib.Path('pt',f.name) if 'pt' in parts else pathlib.Path(f.name)
            if f.name.upper().startswith(('LICENSE','COPYING','NOTICE')):
                rel=pathlib.Path(('aether-' if name.startswith('aether') else 'sing-box-')+f.name)
            dst=destination/rel; dst.parent.mkdir(parents=True,exist_ok=True); shutil.copy2(f,dst)
            if dst.suffix not in ('.txt','.md','.json','.dll'): dst.chmod(0o755)

def main():
    parser=argparse.ArgumentParser(); parser.add_argument('platform',choices=['macos-arm64','windows-x86_64','android']); parser.add_argument('--output',type=pathlib.Path,help='Desktop runtime staging directory'); args=parser.parse_args()
    if args.platform=='android':
        for abi,arch in [('arm64-v8a','arm64'),('armeabi-v7a','armv7'),('x86_64','x86_64')]:
            data,sha=archive('CluvexStudio/Aether',f'aether-android-{arch}.tar.gz')
            with tempfile.TemporaryDirectory() as tmp:
                dest=pathlib.Path(tmp); extract(data,'aether.tar.gz',dest)
                out=ROOT/'android/app/src/main/jniLibs'/abi; out.mkdir(parents=True,exist_ok=True)
                hashes={}
                for f in dest.rglob('*'):
                    if f.is_file() and (f.name=='aether' or f.parent.name=='pt') and f.suffix not in ('.txt','.md','.json'):
                        target=out/('lib'+f.name.replace('-','_')+'.so'); shutil.copy2(f,target); target.chmod(0o755)
                        hashes[target.name]=hashlib.sha256(target.read_bytes()).hexdigest()
                assets=ROOT/'android/app/src/main/assets/runtime'; assets.mkdir(parents=True,exist_ok=True)
                (assets/(abi+'.json')).write_text(json.dumps(hashes,indent=2))
        data,sha=archive('heiher/sockstun','hev.sockstun-8.0-release.apk')
        import io
        with zipfile.ZipFile(io.BytesIO(data)) as apk:
            for abi in ['arm64-v8a','armeabi-v7a','x86_64']:
                name='libhev-socks5-tunnel.so'; binary=apk.read(f'lib/{abi}/{name}')
                target=ROOT/'android/app/src/main/jniLibs'/abi/name;target.write_bytes(binary);target.chmod(0o755)
                manifest=ROOT/'android/app/src/main/assets/runtime'/(abi+'.json')
                hashes=json.loads(manifest.read_text());hashes[name]=hashlib.sha256(binary).hexdigest();manifest.write_text(json.dumps(hashes,indent=2)+'\n')
        return
    dest=args.output or ROOT/'src-tauri/resources/runtime'; dest.mkdir(parents=True,exist_ok=True)
    pairs=[('CluvexStudio/Aether',f'aether-{args.platform}'+('.zip' if args.platform.startswith('windows') else '.tar.gz')),
           ('SagerNet/sing-box','sing-box-1.14.3-'+('windows-amd64.zip' if args.platform.startswith('windows') else 'darwin-arm64.tar.gz'))]
    sources=[]
    for repo,name in pairs:
        print('Fetching',name,flush=True); data,sha=archive(repo,name); extract(data,name,dest); sources.append({'repo':repo,'tag':VERSIONS[repo],'asset':name,'sha256':sha})
    files={str(f.relative_to(dest)):hashlib.sha256(f.read_bytes()).hexdigest() for f in dest.rglob('*') if f.is_file() and f.name!='manifest.json'}
    (dest/'manifest.json').write_text(json.dumps({'sources':sources,'files':files},indent=2)+'\n')
if __name__=='__main__': main()
