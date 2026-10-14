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
                specs.load(path)

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
