#!/usr/bin/env python3
"""Recompile launcher trust for the owner-approved signing-key migration.

Consumes only the tested beta.17 production artifact. Native libraries, original
game UI, companion hooks, shaders and game assets remain byte-identical.
"""
import argparse, copy, hashlib, json, struct, subprocess, zipfile
from pathlib import Path
from parallel_package import source_built_dex
import parallel_package
from apk_format import axml_chunks, string_pool, make_string_pool, manifest_values

ROOT = Path(__file__).resolve().parents[2]
CONFIG = ROOT/'android/release/release-identity-2026.json'
OLD_SIGNER = 'e2253eb7370c495ef0265efcdbe6bfbe09832070558fe9ed7047e7a2361ad75e'

def digest(data):
    return hashlib.sha256(data).hexdigest()

def fixture_manifest(data):
    parts = axml_chunks(data)
    index = next(i for i,p in enumerate(parts) if struct.unpack_from('<H',p)[0] == 1)
    strings, flags = string_pool(parts[index])
    assert strings.count('1.1.0-beta.17') == 1
    strings = ['1.1.0-beta.16' if s == '1.1.0-beta.17' else s for s in strings]
    parts[index] = bytearray(make_string_pool(strings, flags))
    changed = 0
    for node in parts:
        if struct.unpack_from('<H',node)[0] != 0x102: continue
        if strings[struct.unpack_from('<I',node,20)[0]] != 'manifest': continue
        start,size,count = struct.unpack_from('<HHH',node,24)
        for j in range(count):
            pos = 16+start+j*size
            if strings[struct.unpack_from('<I',node,pos+4)[0]] == 'versionCode':
                assert struct.unpack_from('<I',node,pos+16)[0] == 33
                struct.pack_into('<I',node,pos+16,32); changed += 1
    assert changed == 1
    result = struct.pack('<HHI',3,8,8+sum(map(len,parts)))+b''.join(parts)
    expected = copy.deepcopy(manifest_values(data))
    expected[0]['attrs'].update(versionName='1.1.0-beta.16',versionCode=32)
    assert manifest_values(result) == expected
    return result

def build(candidate, report_path, sdk, output):
    config = json.loads(CONFIG.read_text())
    report = json.loads(report_path.read_text())
    assert report['sha256'] == digest(candidate.read_bytes()) and report['signed'] is False
    assert report['native_build_run'] == config['native_run']
    assert report['native_source_sha'] == config['native_source_sha']
    assert report['packaging_source_sha'] == config['native_source_sha']
    assert report['signer_sha256'] == OLD_SIGNER
    parallel_package.DEV_BRANCH = config['feed_branch']
    dex = source_built_dex(config['signer_sha256'], sdk/'build-tools/36.0.0/zipalign')
    assert config['feed_branch'].encode() in dex and config['signer_sha256'].encode() in dex
    assert OLD_SIGNER.encode() not in dex
    output.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(candidate) as old:
        assert len(old.namelist()) == len(set(old.namelist())) and old.testzip() is None
        meta = json.loads(old.read('assets/rerac-build.json'))
        assert (meta['launcher_version'],meta['version_code']) == ('1.1.0-beta.17',33)
        assert meta['package_id'] == config['package_id'] and meta['signer_sha256'] == OLD_SIGNER
        assert meta['native_source_sha'] == config['native_source_sha'] and meta['native_source_rebuild'] is True
        native = {n:digest(old.read(n)) for n in old.namelist() if n.startswith('lib/') and n.endswith('.so')}
        assert len(native) == 4 and native == report['native_sha256'] == meta['native_sha256']
        meta.update(signer_sha256=config['signer_sha256'],update_feed_branch=config['feed_branch'],
                    signing_identity='release-2026',requires_reinstall_from_legacy_signer=True)
        for fixture in (False,True):
            name = 'upgrade-fixture-unsigned.apk' if fixture else 'beta17-new-baseline-unsigned.apk'
            target = output/name
            assert not target.exists()
            raw = target.with_suffix('.unaligned.apk')
            metadata = dict(meta)
            if fixture: metadata.update(launcher_version='1.1.0-beta.16',version_code=32,upgrade_test_fixture=True)
            with zipfile.ZipFile(raw,'w') as new:
                for entry in old.infolist():
                    assert not entry.filename.upper().endswith(('.RSA','.SF','.DSA','.EC'))
                    body = old.read(entry)
                    if entry.filename == 'classes6.dex': body = dex
                    elif entry.filename == 'assets/rerac-build.json': body = (json.dumps(metadata,sort_keys=True,separators=(',',':'))+'\n').encode()
                    elif fixture and entry.filename == 'AndroidManifest.xml': body = fixture_manifest(body)
                    clone = copy.copy(entry); clone.extra = b''
                    new.writestr(clone,body)
            subprocess.run([str(sdk/'build-tools/36.0.0/zipalign'),'-P','16','-f','4',str(raw),str(target)],check=True)
            raw.unlink()
            with zipfile.ZipFile(target) as checked:
                assert checked.namelist() == old.namelist() and checked.testzip() is None
                changes = {'classes6.dex','assets/rerac-build.json'} | ({'AndroidManifest.xml'} if fixture else set())
                for n in old.namelist():
                    if n not in changes: assert digest(old.read(n)) == digest(checked.read(n)), n
                assert {n:digest(checked.read(n)) for n in native} == native
            proof = dict(schema=1,apk=name,sha256=digest(target.read_bytes()),bytes=target.stat().st_size,
                         source_unsigned_sha256=report['sha256'],native_sha256=native,
                         changed_entries=sorted(changes),identity=config,upgrade_test_fixture=fixture)
            target.with_suffix('.json').write_text(json.dumps(proof,indent=2)+'\n')
            print(name,proof['sha256'])

if __name__ == '__main__':
    p = argparse.ArgumentParser()
    for name in ('candidate','report','sdk','output'): p.add_argument('--'+name,type=Path,required=True)
    args = p.parse_args(); build(args.candidate,args.report,args.sdk,args.output)
