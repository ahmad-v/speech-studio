# Offline manual | Speech Studio

Development source only. Launch the GUI after compiling. Choose an operation, enter local input path and a new output path where needed, set the parameter as described under the operation selector, then Run. Status and errors are shown below. Cancel requests cooperative cancellation. The draft editor accepts transcript or plain text. Choose a theme to change workbench colors and control borders. UI scale adjusts the controls from 80% to 300% and remembers your appearance choices when you reopen the app. Paths are manually typed in this increment; native dialogs and full accessibility remain pending.

Launch `speech-studio --audio` to open the separate native audio workspace. Use **Open WAV…** to select a mono or stereo PCM16 WAV (16, 32, 44.1 or 48 kHz; at most ten minutes and 128 MiB). The worker validates and loads it off the UI thread, draws a peak waveform with a moving playhead, and supports play, pause/resume, stop, slider seeking, and clicking the waveform to seek. The system audio device is opened only when playback starts. Open an SRT transcript or a version 1 JSON transcript project with **Open SRT / project…**. The cue list highlights the cue at the current playback position; selecting a cue seeks to its start. Edit the selected cue's start and end in `HH:MM:SS,mmm` format, then apply. Invalid intervals and changes that break transcript order are rejected. Undo and redo retain up to 20 timing edits. **Save project…** writes the cues to a new JSON file using the same `schema_version: 1` format as `export-json`; audio paths are deliberately not stored. Existing files are never overwritten. Transcript input is limited to 16 MiB and 20,000 cues. The WAV decoder fixture passed in GitHub Integration; the focused waveform seek unit test and strict headless Clippy passed locally. Device output and platform audio backends remain unverified on physical hardware.

- `open-vtt`: Imports simple UTF-8 cues. Cue settings, regions, styles, and timed markup are outside this route. Cue identifiers and NOTE blocks are omitted.
- `open-lrc`: Imports UTF-8 LRC metadata, signed `[offset:milliseconds]`, timestamps such as `[01:23.45]`, and repeated timestamps on one lyric line. Cue ends are inferred from the next timestamp; the final line receives a 3-second duration. Review timings before using the draft.
- `prepare-wav`: Converts mono or stereo PCM16 WAV at 16, 32, or 48 kHz to a new 16 kHz mono WAV. Downsampling averages groups of input frames.
- `audio-info`: 16 kHz mono signed PCM16 WAV.
- `play-wav`: Plays a local mono or stereo PCM16 WAV at 16, 32, 44.1 or 48 kHz, up to ten minutes and 128 MiB. Select `Play WAV audio`, browse to the file, then Run. The output path and parameter are unused. Cancel stops playback. The default edition supports WAV playback; device availability depends on the host audio system.
- `detect-voice-activity`: Reads a 16 kHz mono PCM16 WAV and writes a new JSON report of energy-active intervals. This short-time RMS detector is an estimate. Tones, music and noise may count as activity; quiet or unvoiced speech may be missed. It is not a neural speech detector and does not identify speakers.
- `analyze-transcript`: Reads the SRT input or current draft and writes a new JSON report with an estimated language, classifier confidence, Unicode word/character/grapheme counts, sentences, non-empty text lines, unique-word ratio, cue-covered duration and an estimated words-per-cue-minute rate. The language confidence belongs to the local text classifier, not speech recognition. No ASR confidence is reported because the current decoder does not provide one. Rate estimates use subtitle cue timing, not acoustic speech duration.
- `open-srt`: UTF-8 SRT subset; editable draft.
- `replace-text`: Enter JSON such as `{"find":"old","replace":"new"}`. The operation uses the current draft, or a supplied SRT if the draft is empty, and returns an edited draft. Matches are case-sensitive and confined to individual cues. Timings are preserved. Save explicitly to export the changes.
- `export-srt`: Validated SRT output.
- `export-vtt` writes WebVTT from SRT input or an edited draft. Cue text is exported as literal text.
- `export-lrc` writes timestamped lyric lines with centisecond start times. LRC does not encode cue end times, so timing duration is lost; multiline cue text is joined with spaces.
- `transcribe`: For an ASR-enabled build, leave the parameter blank to use the [private English model pack](WHISPER_MODEL_PACK.md).
- `export-json`: Save transcript text and timings from the SRT path or draft. The schema is `{"schema_version":1,"segments":[{"start_ms":0,"end_ms":1000,"text":"example"}]}`.
- `open-json`: Restore a version 1 transcript into an editable SRT draft. Unsupported versions, unknown fields, invalid timing and text outside the SRT draft subset are rejected.
- `export-txt`: Plain transcript text.

Limitations: No redistributed ASR model or reference speech. Optional native inference source only, not enabled in default build. Private Tiny English pack or explicit local model path with expected SHA-256 required; user hash is integrity, not authenticity. Cancellation cannot interrupt inference itself. Playback, language accuracy, WER, translation, diarization, TTS and model packaging remain unverified. The bundled energy VAD is not neural and cannot distinguish speech from other sustained sounds. Transcript language identification and linguistic counts are estimates, not ground truth. SRT supports strict basic timecodes and sequential indices; overlapping segments allowed.

No output overwrite is allowed. No accounts, ads, telemetry or runtime downloads are intended. Do not use this unverified snapshot with valuable or hostile input. Citation and donations are voluntary.

WAV preparation downmixes and uses weighted frame averaging for supported PCM16 inputs. High-frequency content can alias during downsampling.

- `shift-timing`: Enter a signed offset in milliseconds, such as `-200`. The operation returns a new SRT draft with the same cue text and durations. A shift outside the SRT timestamp range fails. Export the draft to save it.
- `waveform-svg`: Supply a 16 kHz mono PCM16 WAV and a new SVG path. The export shows peak amplitudes and duration for clips up to ten minutes. Use Prepare WAV first for other supported PCM16 input rates.

## Additional operations

- `scale-timing`. Enter a whole percentage from 1 to 1000. For example, 105 makes times 5% later relative to zero. Times are rounded to the nearest millisecond.
- `trim-cues`. Enter `start_ms,end_ms`. Intersecting cues are clipped to the range, and remaining times keep the original time origin.
- `split-cue`. Enter JSON such as `{"cue":1,"at_ms":2500,"first":"First part","second":"Second part"}`. The split point must be inside the cue.
- `merge-cues`. Enter first and last cue numbers, such as `2,4`. Text is separated by newlines and the merged cue ends at the latest selected end time.
- `delete-cue`. Enter the 1-based cue number. At least one cue must remain.

Transcript cue edits operate on a validated SRT draft and can optionally write a new `.srt` file:

- `replace-cue-text`: JSON `{"cue":1,"text":"New words"}` replaces one cue while preserving its time.
- `insert-cue`: JSON `{"index":2,"start_ms":2000,"end_ms":2800,"text":"Inserted"}` inserts at a 1-based position and checks time order and range.
- `set-cue-timing`: JSON `{"cue":1,"start_ms":1000,"end_ms":2500}` changes one cue's interval; end must be after start and timestamps must remain ordered.
- `shift-cue-range`: JSON `{"first":2,"last":4,"offset_ms":-100}` shifts a 1-based inclusive cue range and rejects underflow, overflow or order violations.
- `remove-duplicate-cues` removes only exact repeated records with the same text and both timestamps; it keeps the first.
- `normalize-cue-whitespace` collapses whitespace within each cue line and preserves nonempty line boundaries. `join-cue-lines` flattens all cue lines to one space-separated line.
- `wrap-cue-text`: JSON `{"max_graphemes":42}` wraps at word boundaries using Unicode grapheme counts. A single long word is preserved intact.
- `swap-cue-text`: JSON `{"first":1,"second":2}` swaps text while leaving the two time intervals fixed.
- `replace-text-range`: JSON `{"first":1,"last":3,"find":"old","replace":"new"}` performs case-sensitive literal replacement in an inclusive cue range.

The preceding cue edits return an SRT draft. You can also supply a new .srt output path to save the edit immediately.

## Cue text tools

These ten tools edit an inclusive, 1-based cue range using JSON parameters. The range is limited to the existing transcript, timestamps are preserved, every cue must remain nonempty, and the rendered result stays within 2 MiB:

- `uppercase-cue-range`, `lowercase-cue-range`, `title-case-cue-range`, `sentence-case-cue-range`, and `trim-cue-range`: `{"first":1,"last":3}`. Case tools support Unicode text; title/sentence case normalize spaces within each cue line.
- `prefix-cue-range` and `suffix-cue-range`: `{"first":1,"last":3,"text":"[Speaker] "}` adds the literal text to each cue in range.
- `reverse-cue-lines-range`, `sort-cue-lines-range`, and `deduplicate-cue-lines-range`: `{"first":1,"last":3}` reverses, Unicode-lexically sorts, or keeps the first exact occurrence of each line within each selected cue.

An edit that would produce a blank cue, exceed the transcript limit, or create invalid SRT is rejected without writing output.
