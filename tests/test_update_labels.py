import unittest
from test_native_settings import settings


class UpdateLabels(unittest.TestCase):
    def test_updater_codes_are_displayed_as_human_text(self):
        labels=settings.update_state
        self.assertEqual(labels.nodalix_status_label({'status':'up_to_date','message':'up_to_date'}),'Nodalix está actualizado')
        self.assertEqual(labels.nodalix_status_label({'status':'update_available','latest_version':'0.4.1'}),'Nueva versión: 0.4.1')
        for status in ('no_release','no_compatible_release','manifest_missing','invalid_manifest','network_error'):
            self.assertNotIn(status,labels.nodalix_status_label({'status':status}))
        self.assertEqual(labels.nodalix_status_label({'status':'new_code','message':'new_code'}),'Comprobación de Nodalix completada')
