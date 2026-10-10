#!/usr/bin/env python3
"""Verify packaged runtime files on both native and cross-platform hosts."""
import argparse,hashlib,json,pathlib
p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);a=p.parse_args()
manifest=json.loads((a.root/'manifest.json').read_text())
for name,expected in manifest['files'].items():
    relative=pathlib.PurePosixPath(name.replace('\\','/'))
    windows=pathlib.PureWindowsPath(name)
    if relative.is_absolute() or windows.drive or '..' in relative.parts:raise SystemExit('Unsafe runtime path')
    binary=a.root.joinpath(*relative.parts)
    if hashlib.sha256(binary.read_bytes()).hexdigest()!=expected:raise SystemExit('Runtime checksum mismatch: '+name)
print('Verified',len(manifest['files']),'packaged runtime files')
