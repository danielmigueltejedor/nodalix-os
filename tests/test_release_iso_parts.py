import functools, hashlib, http.server, importlib.util, json, sys, tempfile, threading, unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
from release_iso import iso_metadata, publication_assets
spec=importlib.util.spec_from_file_location('iso_downloader',ROOT/'tools/nodalix-download-iso.py')
downloader=importlib.util.module_from_spec(spec);spec.loader.exec_module(downloader)

class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self,*args):pass

class ReleaseIsoPartsTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.root=Path(self.temp.name)
        self.image=self.root/'nodalix-0.3.0-x86_64.iso'
        self.payload=bytes(range(256))*77
        self.image.write_bytes(self.payload)
    def tearDown(self):self.temp.cleanup()
    def serve(self):
        handler=functools.partial(QuietHandler,directory=str(self.root))
        server=http.server.ThreadingHTTPServer(('127.0.0.1',0),handler)
        thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
        self.addCleanup(server.server_close);self.addCleanup(server.shutdown)
        return f'http://127.0.0.1:{server.server_port}'
    def test_complete_bytes_survive_parts_and_oversized_asset_is_not_published(self):
        meta=iso_metadata(self.image,5000)
        self.assertEqual(meta['sha256'],hashlib.sha256(self.payload).hexdigest())
        self.assertEqual(b''.join((self.root/p['asset']).read_bytes() for p in meta['parts']),self.payload)
        self.assertTrue(all(p['size']<=5000 for p in meta['parts']))
        (self.root/'nodalix-manifest.json').write_text(json.dumps({'iso':meta}))
        names={p.name for p in publication_assets(self.root)}
        self.assertNotIn(self.image.name,names)
        self.assertTrue({p['asset'] for p in meta['parts']}.issubset(names))
    def test_small_images_retain_single_asset_compatibility(self):
        meta=iso_metadata(self.image,len(self.payload))
        self.assertEqual(meta['asset'],self.image.name)
        self.assertNotIn('parts',meta)
        result=downloader.download_iso({'iso':meta},self.serve(),self.root/'downloaded.iso')
        self.assertEqual(result.read_bytes(),self.payload)
    def test_real_http_multipart_download_produces_verified_complete_image(self):
        meta=iso_metadata(self.image,5000)
        result=downloader.download_iso({'iso':meta},self.serve(),self.root/'downloaded.iso')
        self.assertEqual(result.read_bytes(),self.payload)
    def test_corrupted_part_never_creates_final_image(self):
        meta=iso_metadata(self.image,5000)
        (self.root/meta['parts'][1]['asset']).write_bytes(b'corrupt')
        target=self.root/'downloaded.iso'
        with self.assertRaises(ValueError):downloader.download_iso({'iso':meta},self.serve(),target)
        self.assertFalse(target.exists())
        self.assertFalse(list(self.root.glob('.nodalix-iso-*')))
    def test_valid_parts_with_wrong_whole_hash_are_rejected(self):
        meta=iso_metadata(self.image,5000);meta['sha256']='0'*64
        target=self.root/'downloaded.iso'
        with self.assertRaises(ValueError):downloader.download_iso({'iso':meta},self.serve(),target)
        self.assertFalse(target.exists())
    def test_personal_files_are_preserved_and_unsafe_part_names_rejected(self):
        meta=iso_metadata(self.image,5000);target=self.root/'downloaded.iso';target.write_bytes(b'personal')
        with self.assertRaises(FileExistsError):downloader.download_iso({'iso':meta},'https://example.invalid',target)
        self.assertEqual(target.read_bytes(),b'personal')
        meta['parts'][0]['asset']='../outside'
        with self.assertRaises(ValueError):downloader.download_iso({'iso':meta},'https://example.invalid',self.root/'new.iso')

if __name__=='__main__':unittest.main()
