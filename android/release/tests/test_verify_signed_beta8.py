"""Public receipt/feed rejection tests; the fixture signer is never a release key."""
import base64,datetime,importlib.util,json,sys,unittest
from pathlib import Path
from unittest.mock import patch
from cryptography import x509
from cryptography.hazmat.primitives import hashes,serialization
from cryptography.hazmat.primitives.asymmetric import padding,rsa
from cryptography.x509.oid import NameOID
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import verify_signed_beta8 as verify

class FeedValidationTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.key=rsa.generate_private_key(public_exponent=65537,key_size=2048)
        name=x509.Name([x509.NameAttribute(NameOID.COMMON_NAME,"Disposable beta8 verifier test")])
        now=datetime.datetime.now(datetime.timezone.utc)
        cls.cert=(x509.CertificateBuilder().subject_name(name).issuer_name(name).public_key(cls.key.public_key())
            .serial_number(1).not_valid_before(now-datetime.timedelta(minutes=1)).not_valid_after(now+datetime.timedelta(days=1))
            .sign(cls.key,hashes.SHA256())).public_bytes(serialization.Encoding.DER)
        cls.pub=cls.key.public_key().public_bytes(serialization.Encoding.DER,serialization.PublicFormat.SubjectPublicKeyInfo)
        cls.cert_sha=verify.digest(cls.cert)
    def setUp(self):
        self.pin=patch.object(verify,"SIGNER",self.cert_sha);self.pin.start();self.addCleanup(self.pin.stop)
        self.release=dict(package_id=verify.PACKAGE,signer_sha256=self.cert_sha,launcher_version=verify.VERSION,
            core_version="0.3.5",version_code=24,bytes=123,sha256="a"*64,min_sdk=26,abis=["arm64-v8a","x86_64"],apk_url=verify.URL)
    def envelope(self):
        body=json.dumps(dict(schema=1,channel="development",release=self.release),separators=(",",":")).encode()
        return json.dumps(dict(algorithm="SHA256withRSA",payload=base64.b64encode(body).decode(),
            signature=base64.b64encode(self.key.sign(body,padding.PKCS1v15(),hashes.SHA256())).decode()))
    def check(self,text):
        return verify.feed_check(text,self.cert,self.pub,"a"*64,123)
    def test_sdk_public_certificate_extraction(self):
        pem=x509.load_der_x509_certificate(self.cert).public_bytes(serialization.Encoding.PEM).decode()
        self.assertEqual(verify.sdk_certificate(pem),(self.cert,self.pub))
    def test_sdk_rejects_multiple_certificates(self):
        pem=x509.load_der_x509_certificate(self.cert).public_bytes(serialization.Encoding.PEM).decode()
        with self.assertRaisesRegex(ValueError,"one SDK"):verify.sdk_certificate(pem+pem)
    def test_valid_exact_candidate_feed(self):
        self.assertEqual(self.check(self.envelope())["release"],self.release)
    def test_rejects_wrong_release_identity(self):
        for field,value in [("version_code",23),("package_id","another.app"),("apk_url",verify.URL+"x"),("sha256","b"*64),("bytes",124)]:
            with self.subTest(field=field):
                original=self.release[field];self.release[field]=value
                with self.assertRaisesRegex(ValueError,"Feed mismatch"):self.check(self.envelope())
                self.release[field]=original
    def test_rejects_tampered_signature(self):
        obj=json.loads(self.envelope());obj["signature"]=base64.b64encode(b"invalid").decode()
        with self.assertRaises(Exception):self.check(json.dumps(obj))
    def test_rejects_wrong_certificate_pin(self):
        with patch.object(verify,"SIGNER","b"*64):
            with self.assertRaises(ValueError):self.check(self.envelope())
    def test_rejects_wrong_apk_public_key(self):
        with self.assertRaisesRegex(ValueError,"public-key mismatch"):
            verify.feed_check(self.envelope(),self.cert,b"wrong","a"*64,123)
    def test_rejects_wrong_channel(self):
        obj=json.loads(self.envelope());body=json.loads(base64.b64decode(obj["payload"]));body["channel"]="stable"
        obj["payload"]=base64.b64encode(json.dumps(body).encode()).decode()
        with self.assertRaisesRegex(ValueError,"channel"):self.check(json.dumps(obj))
if __name__=="__main__":unittest.main()
