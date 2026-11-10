"""The animated background gallery must be laid out like the static one."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
SOURCE = (ROOT/'gnome/control-center/cc-nodalix-background.c').read_text()
BUILDER = (ROOT/'tools/build-settings-packages.py').read_text()


class AnimatedGalleryTests(unittest.TestCase):
    def test_tiles_have_a_fixed_intrinsic_size_instead_of_the_image_size(self):
        # GtkPicture reports its image's pixel size, so a 480 px preview made each
        # tile 480 px wide and the grid collapsed to one wallpaper per row.
        self.assertNotIn('gtk_picture_new_for_filename', SOURCE)
        self.assertIn('gtk_picture_new_for_paintable', SOURCE)
        self.assertRegex(SOURCE, r'#define WIDTH 144\n#define HEIGHT 108')
        self.assertIn('thumbnail_width (GdkPaintable *paintable) { return WIDTH; }', SOURCE)
        self.assertIn('thumbnail_height (GdkPaintable *paintable) { return HEIGHT; }', SOURCE)

    def test_thumbnails_are_cropped_to_cover_the_tile(self):
        self.assertIn('MAX (width / source_width, height / source_height)', SOURCE)
        self.assertIn('gtk_snapshot_push_clip', SOURCE)

    def test_flowbox_matches_the_static_background_chooser(self):
        for call in ('gtk_widget_set_halign (GTK_WIDGET (self->gallery), GTK_ALIGN_CENTER)',
                     'gtk_flow_box_set_homogeneous (self->gallery, TRUE)',
                     'gtk_flow_box_set_min_children_per_line (self->gallery, 1)',
                     'gtk_flow_box_set_max_children_per_line (self->gallery, 8)',
                     'gtk_flow_box_set_row_spacing (self->gallery, 12)',
                     'gtk_flow_box_set_column_spacing (self->gallery, 12)',
                     'gtk_flow_box_set_activate_on_single_click (self->gallery, TRUE)'):
            self.assertIn(call, SOURCE)

    def test_tiles_carry_no_caption_like_the_static_ones(self):
        self.assertNotIn('PANGO_ELLIPSIZE_END', SOURCE)
        self.assertIn('gtk_widget_set_tooltip_text (child, details)', SOURCE)

    def test_packaged_and_imported_previews_share_one_shape(self):
        poster = (ROOT/'gnome/settings/wallpaper_poster.py').read_text()
        shape = 'scale=288:216:force_original_aspect_ratio=increase,crop=288:216'
        self.assertIn(shape, poster)
        self.assertIn(shape, BUILDER)
        self.assertNotRegex(BUILDER, r"scale=480")


if __name__ == '__main__':
    unittest.main()
