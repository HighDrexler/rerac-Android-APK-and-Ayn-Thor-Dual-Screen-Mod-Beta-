#!/usr/bin/env python3
"""Apply an offline-approved public APK signing block; no private key enters CI.
Sign with apksigner --alignment-preserved true before creating an approval.
The exact unsigned/signed hashes make approvals unusable for other APKs.
"""
import argparse, base64, hashlib, json, re, struct, subprocess, zipfile
from pathlib import Path
SIGNER = '25c5eebb05a013bf3fb84706f9ad87904b8d4d8a99ba7857d62e665d2b94e24a'
def sha(data): return hashlib.sha256(data).hexdigest()
def require(ok, message):
    if not ok: raise ValueError(message)
def layout(data):
    e = data.rfind(b'PK\x05\x06', max(0, len(data)-65557))
    require(e >= 0 and e+22 <= len(data), 'Missing ZIP end record')
    _, d, dc, n, nt, size, offset, comment = struct.unpack_from('<4s4H2IH', data, e)
    require(d == dc == 0 and n == nt and nt < 65535, 'Unsupported ZIP layout')
    require(e+22+comment == len(data) and offset+size == e, 'ZIP offsets invalid')
    return e, offset

def reconstruct(unsigned, approval):
    require(sha(unsigned) == approval['unsigned_sha256'], 'Unsigned APK hash mismatch')
    e, cd = layout(unsigned)
    require(cd == approval['central_directory_offset'], 'Unexpected central directory')
    insertion = base64.b64decode(approval['public_signing_block'], validate=True)
    require(32 <= len(insertion) <= 131072 and insertion[-16:] == b'APK Sig Block 42', 'Invalid public signing block')
    size = struct.unpack_from('<Q', insertion, len(insertion)-24)[0]+8
    start = len(insertion)-size
    require(0 <= start < 4096 and insertion[:start] == b'\0'*start, 'Invalid alignment padding')
    require(size >= 32 and struct.unpack_from('<Q', insertion, start)[0] == size-8, 'Invalid signing block size')
    out = bytearray(unsigned[:cd]+insertion+unsigned[cd:])
    struct.pack_into('<I', out, e+len(insertion)+16, cd+len(insertion))
    require(len(out) == approval['signed_bytes'] and sha(out) == approval['signed_sha256'], 'Approved APK reconstruction failed')
    return out

def verify(path, tools):
    result = subprocess.check_output([str(tools/'apksigner'), 'verify', '--verbose', '--print-certs', str(path)], text=True)
    require('Signer #1 certificate SHA-256 digest: '+SIGNER in result, 'Different APK signer')
    require('Verified using v2 scheme (APK Signature Scheme v2): true' in result, 'Missing v2 signature')
    subprocess.run([str(tools/'zipalign'), '-c', '-P', '16', '4', str(path)], check=True, stdout=subprocess.DEVNULL)
    return result

def make(unsigned, signed, tools):
    verify(signed, tools)
    u, s = unsigned.read_bytes(), signed.read_bytes()
    ue, uc = layout(u); se, sc = layout(s)
    require(u[:uc] == s[:uc] and u[uc:ue] == s[sc:se], 'Sign using --alignment-preserved true')
    a = dict(unsigned_name=unsigned.name, signed_name=signed.name, unsigned_sha256=sha(u), signed_sha256=sha(s),
             signed_bytes=len(s), central_directory_offset=uc, public_signing_block=base64.b64encode(s[uc:sc]).decode())
    require(bytes(reconstruct(u, a)) == s, 'Signature approval is not lossless')
    with zipfile.ZipFile(signed) as z: a['build'] = json.loads(z.read('assets/rerac-build.json'))
    return a

def apply(approval_file, artifacts, output, tools):
    doc = json.loads(approval_file.read_text())
    require(doc['schema'] == 1 and 1 <= len(doc['apks']) <= 2, 'Invalid approval')
    output.mkdir(parents=True, exist_ok=True)
    for a in doc['apks']:
        require(re.fullmatch(r'ReRAC-Launcher-1\.1\.0-beta\.[1-9][0-9]*-unsigned\.apk', a['unsigned_name']) is not None, 'Invalid unsigned filename')
        require(a['signed_name'] == a['unsigned_name'].replace('-unsigned.apk', '.apk'), 'Invalid signed filename')
        found = list(artifacts.rglob(a['unsigned_name']))
        require(len(found) == 1, 'Exact candidate artifact required')
        target = output/a['signed_name']; require(not target.exists(), 'Refusing to overwrite signed APK')
        target.write_bytes(reconstruct(found[0].read_bytes(), a))
        try: verify(target, tools)
        except Exception:
            target.unlink(); raise
        print(a['signed_sha256'], target.name)
    (output/'SHA256SUMS.txt').write_text(''.join(a['signed_sha256']+'  '+a['signed_name']+'\n' for a in doc['apks']))

if __name__ == '__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--approval',type=Path,required=True);p.add_argument('--artifacts',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--tools',type=Path,required=True)
    args=p.parse_args(); apply(args.approval,args.artifacts,args.output,args.tools)
