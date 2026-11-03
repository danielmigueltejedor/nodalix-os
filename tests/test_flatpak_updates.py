import importlib.util
from pathlib import Path
import unittest
from unittest.mock import Mock, patch
ROOT=Path(__file__).resolve().parents[1]
s=importlib.util.spec_from_file_location('fp_updates',ROOT/'updater/flatpak_updates.py');fp=importlib.util.module_from_spec(s);s.loader.exec_module(fp)


class FlatpakUpdates(unittest.TestCase):
    def test_only_origins_of_installed_refs_are_queried(self):
        ref=Mock();ref.get_origin.return_value='flathub';ref.format_ref.return_value='app/qa/x86_64/stable'
        remote=Mock();remote.get_name.return_value='flathub';remote.get_disabled.return_value=False;remote.get_url.return_value='https://example.invalid/repo'
        broken=Mock();broken.get_name.return_value='debug-origin'
        installation=Mock();installation.list_installed_refs.return_value=[ref];installation.list_remotes.return_value=[remote,broken];installation.get_is_user.return_value=True
        groups,manual=fp.origin_groups(installation)
        self.assertEqual(groups,[('--user','flathub',['app/qa/x86_64/stable'])]);self.assertEqual(manual,[])

    def test_ref_without_app_prefix_matches_but_other_refs_do_not(self):
        run=Mock(return_value=Mock(returncode=0,stdout='qa/x86_64/stable\nother/x86_64/stable\n'))
        result=fp.query_group(('--user','flathub',['app/qa/x86_64/stable']),run)
        self.assertEqual(result['updates'],['app/qa/x86_64/stable'])

    def test_one_failed_origin_does_not_block_successful_update(self):
        state={'partial':True,'detail':'partial','origins':[{'scope':'--user','origin':'ok','updates':['app/qa/x86_64/stable']},{'scope':'--user','origin':'bad','updates':[]}]}
        run=Mock(return_value=Mock(returncode=0))
        with patch.object(fp,'check',return_value=state):self.assertEqual(fp.update('all',run),1)
        self.assertEqual(run.call_count,1)
        self.assertEqual(run.call_args.args[0][-1],'app/qa/x86_64/stable')

    def test_local_bundle_without_repo_is_reported_without_query(self):
        ref=Mock();ref.get_origin.return_value='local';ref.format_ref.return_value='app/local/x86_64/stable'
        remote=Mock();remote.get_name.return_value='local';remote.get_disabled.return_value=False;remote.get_url.return_value='file:///nonexistent/nodalix-qa-cache'
        installation=Mock();installation.list_installed_refs.return_value=[ref];installation.list_remotes.return_value=[remote]
        groups,manual=fp.origin_groups(installation)
        self.assertEqual(groups,[]);self.assertEqual(len(manual),1)
