"""Offline tests on invented geometry; no game files or network needed."""
import io
import unittest
import xml.etree.ElementTree as ET
from PIL import Image

from extract_native_refs import render


class RenderingTests(unittest.TestCase):
    def test_stretched_path_is_local_before_matrix_then_canvas_offset(self):
        template = ET.fromstring('''<DataTemplate xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"><Viewbox>
          <Canvas Width="32" Height="32">
            <Path Canvas.Left="18" Canvas.Top="6" Stretch="Fill" Width="12" Height="4"
              Data="F1M30,22h6v2h-6z" Fill="#FF12AB34">
              <Path.RenderTransform><MatrixTransform Matrix="0,1,-1,0,0,0"/></Path.RenderTransform>
            </Path>
          </Canvas></Viewbox></DataTemplate>''')
        im = Image.open(io.BytesIO(render(template, 32))).convert('RGBA')
        self.assertEqual(im.getbbox(), (14, 6, 18, 18))
        self.assertEqual(im.getpixel((15, 10)), (18, 171, 52, 255))


class IdentityTests(unittest.TestCase):
    def test_blank_background_is_not_identity_evidence(self):
        import numpy as np
        from audit_native_refs import dice
        blanks = np.zeros((1, 4), dtype=np.float32)
        self.assertEqual(dice(blanks, blanks)[0, 0], 0.0)


class PackTests(unittest.TestCase):
    def test_hud_mask_checks_pixels_before_assigning_an_identity(self):
        import hashlib
        from pathlib import Path
        from tempfile import TemporaryDirectory
        from extract_hud_refs import read_mask
        with TemporaryDirectory() as tmp:
            mask = Image.new('RGB', (256, 256)); mask.putpixel((2, 1), (255, 0, 0)); mask.putpixel((3, 1), (0, 255, 0))
            key = {'catalog_id': 'test', 'export_png': 'named.png', 'texture_hash': '0x0000000000000001', 'ink': '45acc8', 'rgb_sha256': hashlib.sha256(mask.tobytes()).hexdigest()}
            root = Path(tmp); path = root / (key['texture_hash'] + '.png'); mask.save(path)
            image = read_mask(root, key)
            self.assertEqual([image.getpixel((x, 1)) for x in (1, 2, 3)], [(43,43,43), (69,172,200), (237,237,237)])
            mask.putpixel((3, 1), (0,0,0)); mask.save(path)
            with self.assertRaisesRegex(ValueError, 'unreviewed pixels'):
                read_mask(root, key)

    def test_unreviewed_input_leaves_no_pack(self):
        from pathlib import Path
        import tempfile
        from extract_native_refs import extract
        with tempfile.TemporaryDirectory() as scratch:
            path = Path(scratch) / 'input.xaml'
            path.write_text('<ResourceDictionary/>')
            out = path.parent / 'out'
            with self.assertRaisesRegex(ValueError, 'unreviewed dictionary'):
                extract(path, out)
            self.assertFalse(out.exists())


if __name__ == '__main__':
    unittest.main()
