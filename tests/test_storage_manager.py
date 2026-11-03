"""Real filesystem fixtures for storage measurement and Trash safety."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch

ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('storage_manager',ROOT/'gnome/settings/storage_manager.py')
storage=importlib.util.module_from_spec(spec);spec.loader.exec_module(storage)


class StorageManagerTests(unittest.TestCase):
    def test_large_documents_media_and_development_are_sorted_and_deduplicated(self):
        with tempfile.TemporaryDirectory() as tmp:
            home=Path(tmp); docs=home/'Documents'; music=home/'Music'; dev=home/'cache'
            for folder in (docs,music,dev):folder.mkdir()
            def file(path,size):
                with path.open('wb') as out:out.truncate(size)
            file(docs/'Large.pdf',40*1024**2);file(docs/'Medium.pdf',20*1024**2);file(docs/'Small.pdf',1024)
            os.link(docs/'Large.pdf',docs/'Duplicate.pdf')
            (docs/'outside').symlink_to(home)
            file(music/'Song.flac',2048);file(dev/'compiler',4096)
            rows,totals,partial=storage.file_inventory({docs:'DOCUMENTS',music:'MUSIC'},{dev})
            self.assertFalse(partial)
            self.assertEqual([r['size'] for r in rows['documents']],[40*1024**2,20*1024**2])
            self.assertEqual(totals['documents'],60*1024**2+1024)
            self.assertEqual(rows['music'][0]['name'],'Song.flac')
            self.assertEqual(rows['development'][0]['size'],4096)

    def test_large_tree_does_not_hide_other_categories(self):
        with tempfile.TemporaryDirectory() as tmp:
            docs=Path(tmp)/'Documents';download=Path(tmp)/'Downloads'
            docs.mkdir();download.mkdir()
            for i in range(20):(docs/str(i)).write_text('small')
            (download/'file.zip').write_bytes(b'zip')
            rows,_,partial=storage.file_inventory({docs:'DOCUMENTS',download:'DOWNLOAD'},set(),limit=10)
            self.assertTrue(partial)
            self.assertEqual(rows['downloads'][0]['name'],'file.zip')

    def test_trash_rejects_root_symlink_and_outside_then_uses_gio_for_selected_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);docs=root/'Documents';docs.mkdir()
            inside=docs/'inside.pdf';inside.touch();outside=root/'outside.pdf';outside.touch()
            link=docs/'link.pdf';link.symlink_to(outside)
            with patch.object(storage,'locations',return_value={docs:'DOCUMENTS'}), patch.object(storage,'dev_locations',return_value=set()), patch.object(storage.Gio.File,'new_for_path') as file:
                for forbidden in (docs,outside,link):
                    with self.assertRaises(ValueError):storage.trash_file(forbidden)
                file.assert_not_called()
                storage.trash_file(inside)
                file.assert_called_once_with(str(inside));file.return_value.trash.assert_called_once_with(None)

    def test_system_package_size_matches_desktop_not_name_guess(self):
        with tempfile.TemporaryDirectory() as tmp:
            db=Path(tmp); pkg=db/'example-1.0-1';pkg.mkdir()
            (pkg/'desc').write_text('%NAME%\nexample\n\n%SIZE%\n123456\n')
            (pkg/'files').write_text('%FILES%\nusr/share/applications/org.example.App.desktop\nusr/bin/example\n')
            self.assertEqual(storage.package_sizes(db),{'org.example.App.desktop':('example',123456)})
