import json
import unittest
from pathlib import Path

import numpy as np

from pipeline import spec as specs
from pipeline import timeline, voice

ROOT = Path(__file__).resolve().parent.parent


def words(text, step=10):
    return [{"text": w, "start": i * step, "end": i * step + step - 2} for i, w in enumerate(text.split())]


class SpecTests(unittest.TestCase):
    def test_example_specs_validate(self):
        for path in (ROOT / "specs").glob("*.json"):
            with self.subTest(path=path.name):
                specs.load(path, footage=path.name.startswith("footage"))

    def test_reports_every_problem(self):
        bad = {**specs.DEFAULTS, "title": "x", "scenes": [{"type": "list", "narration": "hi"}, {"type": "nope"}]}
        with self.assertRaises(specs.SpecError) as ctx:
            specs.validate(bad)
        msg = str(ctx.exception)
        self.assertIn("missing 'items'", msg)
        self.assertIn("unknown type 'nope'", msg)
        self.assertIn("should be a 'hook'", msg)

    def test_tree_highlight_path_checked(self):
        scene = {
            "type": "tree",
            "narration": "x",
            "root": {"label": "r", "children": [{"label": "a"}]},
            "highlight": {"path": [3]},
        }
        bad = {**specs.DEFAULTS, "title": "x", "scenes": [{"type": "hook", "title": "t", "narration": "t"}, scene]}
        with self.assertRaisesRegex(specs.SpecError, "does not exist"):
            specs.validate(bad)

    def test_array_text_length_checked(self):
        scene = {"type": "array", "narration": "x", "text": "a" * 20}
        bad = {**specs.DEFAULTS, "title": "x", "scenes": [{"type": "hook", "title": "t", "narration": "t"}, scene]}
        with self.assertRaisesRegex(specs.SpecError, "2 to 16 characters"):
            specs.validate(bad)

    def test_array_cue_matches_shift_words(self):
        scene = {"type": "array", "text": "Hello"}
        cues = timeline.resolve_cues(scene, words("every insert means shifting every byte"), 100)
        self.assertEqual(cues, [30 - timeline.CUE_LEAD])

    def test_tree_cues_cover_levels_and_highlight(self):
        scene = {"type": "tree", "root": {"label": "r", "children": [{"label": "a"}]}, "highlight": {"path": [0]}}
        self.assertEqual(len(specs.cue_targets(scene)), 3)


class CueTests(unittest.TestCase):
    def test_explicit_phrase(self):
        scene = {"type": "list", "items": [{"text": "A", "cue": "second thing"}]}
        cues = timeline.resolve_cues(scene, words("first the second thing here"), 100)
        self.assertEqual(cues, [20 - timeline.CUE_LEAD])

    def test_keyword_stem_match(self):
        scene = {"type": "list", "items": ["Insert anywhere", "Undo is cheap"]}
        cues = timeline.resolve_cues(scene, words("so inserting is fast and undo costs little"), 200)
        self.assertEqual(cues, [10 - timeline.CUE_LEAD, 50 - timeline.CUE_LEAD])

    def test_unmatched_cues_spread_and_ordered(self):
        scene = {"type": "list", "items": ["zzz", "yyy", "xxx"]}
        cues = timeline.resolve_cues(scene, words("nothing matches here at all"), 300)
        self.assertEqual(len(cues), 3)
        self.assertEqual(cues, sorted(cues))
        self.assertTrue(all(b - a >= timeline.MIN_CUE_GAP for a, b in zip(cues, cues[1:])))
        self.assertLessEqual(cues[-1], 300)


class VoiceTests(unittest.TestCase):
    def test_pronunciation_keeps_display_words(self):
        display, spoken = voice.spoken_form("A 100 MB file, O(n) time.", {"MB": "megabyte", "O(n)": "O of n"})
        self.assertEqual(display, ["A", "100", "MB", "file,", "O(n)", "time."])
        self.assertEqual(spoken[2], ["megabyte"])
        self.assertEqual(spoken[4], ["O", "of", "n"])

    def test_tighten_shortens_long_pauses_only(self):
        rate = voice.TTS_RATE
        tone = np.sin(np.arange(rate // 2) / rate * 2 * np.pi * 220).astype(np.float32)
        gap = np.zeros(rate, dtype=np.float32)
        short = np.zeros(rate // 10, dtype=np.float32)
        out = voice._tighten(np.concatenate([tone, gap, tone, short, tone]))
        expected = 3 * len(tone) + len(short) + int(0.26 * rate)
        self.assertLess(abs(len(out) - expected), int(0.05 * rate))


class TimelineTests(unittest.TestCase):
    def test_build_and_srt(self):
        spec = specs.load(ROOT / "specs" / "rope.json")
        narrations = [voice.estimate(s["narration"], 1.0) for s in spec["scenes"]]
        tl, events = timeline.build(spec, narrations, None)
        ends = [s["from"] + s["duration"] for s in tl["scenes"]]
        self.assertEqual(tl["durationInFrames"], ends[-1])
        self.assertEqual([s["from"] for s in tl["scenes"][1:]], ends[:-1])
        for s in tl["scenes"]:
            self.assertTrue(all(0 <= c <= s["duration"] for c in s["cues"]), s["type"])
        self.assertTrue(any(e["kind"] == "whoosh" for e in events))
        srt = timeline.srt(tl)
        self.assertTrue(srt.startswith("1\n00:00:00,000 --> "))
        json.dumps(tl)


if __name__ == "__main__":
    unittest.main()


class WriterTests(unittest.TestCase):
    def test_invalid_draft_gets_one_repair_round(self):
        from unittest import mock

        from pipeline import writer

        good = json.loads((ROOT / "specs" / "rope.json").read_text())
        bad = {**good, "scenes": [{"type": "list", "narration": "x"}]}
        replies = [json.dumps(bad), "Here you go:\n" + json.dumps(good)]
        sent = []

        def ask(client, messages, system=None):
            sent.append(list(messages))
            return replies[len(sent) - 1]

        with mock.patch.object(writer, "_ask", ask), mock.patch.dict("sys.modules", {"anthropic": mock.MagicMock()}):
            spec = writer.write("ropes")
        self.assertEqual(spec["title"], good["title"])
        self.assertEqual(len(sent), 2)
        self.assertIn("does not validate", sent[1][-1]["content"])


class FootageTests(unittest.TestCase):
    def setUp(self):
        from pipeline import footage

        self.footage = footage
        text = "So today I want to talk about ropes. A rope is a tree of strings. Each leaf holds a chunk. That is all for now."
        self.words = [{"text": w, "start": i * 0.4, "end": i * 0.4 + 0.3} for i, w in enumerate(text.split())]
        self.plan = {
            **specs.DEFAULTS,
            "title": "t",
            "format": "landscape",
            "scenes": [
                {"type": "statement", "at": "A rope is a", "until": "tree of strings", "text": "A rope"},
                {"type": "list", "at": "Each leaf holds", "items": ["Leaf", "Chunk"]},
            ],
        }

    def test_scenes_anchor_to_phrases(self):
        tl, events = self.footage.build(self.plan, self.words, 12.0, 30)
        first, second = tl["scenes"]
        self.assertEqual(first["from"], int(round(8 * 0.4 * 30)) - self.footage.LEAD_FRAMES)
        self.assertLessEqual(first["from"] + first["duration"], second["from"])
        self.assertEqual(tl["video"], "speaker.mp4")
        self.assertEqual(len(tl["words"]), len(self.words))
        self.assertTrue(any(e["kind"] == "whoosh" for e in events))

    def test_graphic_opens_just_before_its_first_element(self):
        self.plan["scenes"][1]["items"] = [{"text": "Chunk", "cue": "chunk"}]
        tl, _ = self.footage.build(self.plan, self.words, 12.0, 30)
        self.assertEqual(tl["scenes"][1]["cues"][0], 12)

    def test_missing_phrase_is_reported(self):
        self.plan["scenes"][1]["at"] = "never said this"
        with self.assertRaisesRegex(specs.SpecError, "never said this"):
            self.footage.build(self.plan, self.words, 12.0, 30)

    def test_footage_plans_need_at_not_narration(self):
        specs.validate(self.plan, footage=True)
        del self.plan["scenes"][0]["at"]
        with self.assertRaisesRegex(specs.SpecError, "missing 'at'"):
            specs.validate(self.plan, footage=True)


class BrandTests(unittest.TestCase):
    def test_bundled_brands_load(self):
        from pipeline import brand

        for path in ("brand", "brands/owain", "brands/owain-ink"):
            with self.subTest(path=path):
                brand.load(ROOT / path)

    def test_bad_brand_reports_problems(self):
        import tempfile

        from pipeline import brand

        with tempfile.TemporaryDirectory() as d:
            Path(d, "brand.json").write_text(json.dumps({"colors": {"nope": "#000"}, "style": {"surface": "glass"}, "fonts": {"serif": "missing.woff2"}}))
            with self.assertRaises(brand.BrandError) as ctx:
                brand.load(d)
        msg = str(ctx.exception)
        self.assertIn("unknown colors", msg)
        self.assertIn("style.surface", msg)
        self.assertIn("missing.woff2", msg)

    def test_local_fonts_pass_through(self):
        import tempfile

        from pipeline import brand

        with tempfile.TemporaryDirectory() as d:
            Path(d, "brand.json").write_text(json.dumps({"fonts": {"sans": "local:Helvetica Neue"}, "style": {"corners": True}}))
            published = brand.publish(brand.load(d), Path(d) / "public")
        self.assertEqual(published["fonts"]["sans"], "local:Helvetica Neue")
        self.assertTrue(published["colors"]["corners"])


class LibraryTests(unittest.TestCase):
    def test_library_samples_replace_synthesis(self):
        import subprocess
        import tempfile

        from pipeline import library, sfx

        with tempfile.TemporaryDirectory() as d:
            root = Path(d) / "sounds"
            (root / "pop").mkdir(parents=True)
            subprocess.run(["ffmpeg", "-nostdin", "-loglevel", "error", "-f", "lavfi", "-i", "sine=f=440:d=0.1", str(root / "pop" / "a.wav")], check=True)
            lib = library.SoundLibrary(root, Path(d) / "cache")
            self.assertTrue(lib.has("pop"))
            self.assertFalse(lib.has("whoosh"))
            out = sfx.render([{"t": 0.5, "kind": "pop", "gain": 1.0}, {"t": 1.0, "kind": "whoosh"}], 2.0, 440.0, lib)
            self.assertAlmostEqual(float(np.max(np.abs(out[24000:28800]))), 0.7, places=1)
            self.assertGreater(float(np.max(np.abs(out[48000:]))), 0.0)

    def test_music_bed_loops_to_length(self):
        import subprocess
        import tempfile

        from pipeline import library

        with tempfile.TemporaryDirectory() as d:
            track = Path(d) / "t.wav"
            subprocess.run(["ffmpeg", "-nostdin", "-loglevel", "error", "-f", "lavfi", "-i", "sine=f=220:d=5", str(track)], check=True)
            bed = library.music_bed(track, 12.0, Path(d) / "cache")
        self.assertEqual(len(bed), 12 * 48000)
