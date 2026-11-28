# Offline manual | Speech Studio

Development source only. Launch the GUI after compiling. Choose an operation, enter local input path and a new output path where needed, set the parameter as described under the operation selector, then Run. Status and errors are shown below. Cancel requests cooperative cancellation. The draft editor accepts transcript or plain text. Choose a theme to change workbench colors and control borders. UI scale adjusts the controls from 80% to 300% and remembers your appearance choices when you reopen the app. Paths are manually typed in this increment; native dialogs and full accessibility remain pending.

Launch `speech-studio --audio` to open the separate native audio workspace. Use **Open audio…** to select a WAV, FLAC, MP3, OGG/Vorbis, AAC/M4A or AIFF file (1–8 channels, 8–192 kHz; at most ten minutes and 128 MiB compressed, with a 256 MiB decoded-memory cap). The worker validates and loads it off the UI thread, draws a peak waveform with a moving playhead, and supports play, pause/resume, stop, slider seeking, and clicking the waveform to seek. MP4 audio uses a bounded decoded sample buffer to support accurate seeking. To record, choose an input device from the list, click **Record to WAV…**, and select a new `.wav` path. Recording captures the selected device's default mono or stereo format at 8–96 kHz, converts samples to PCM16 in a bounded background queue, and shows elapsed time. Click **Stop recording** to stop input and finalize the WAV, or **Discard recording** to remove the new recording. Output is opened exclusively, so existing files are preserved. If the device fails or the bounded buffer overflows, the workspace stops and finalizes the partial recording where possible. Physical device enumeration, playback, recording and platform audio behavior remain unverified.

In ASR-enabled builds, the **Offline ASR model** selector verifies the selected app-relative model pack in a background task using its pinned size and SHA-256 before enabling transcription. The default distribution omits model files; build an expansion pack using [the offline model instructions](WHISPER_MODEL_PACK.md) and install it under `engines/<model-id>/` beside the executable. A missing or damaged pack is reported in the workspace without starting inference. Transcription still rechecks the model before loading it, and Whisper cancellation takes effect only before or after native inference, not during the inference call.

Open an SRT transcript or a version 1, 2 or 3 JSON transcript project with **Open SRT / project…**. The cue list highlights the cue at the current playback position; selecting a cue seeks to its aligned start. Type in **Filter transcript cues / find text** to filter the list by case-sensitive text. Select a cue to edit its multiline text and its start/end times, then choose **Apply cue text** or **Apply timing**. **Replace selected** changes literal text in the selected cue; **Replace all** updates every cue. Transcript edits preserve cue times unless timing is explicitly changed, reject empty or invalid results, and share a bounded 20-entry Undo/Redo history with transcript offset changes. Use **Transcript offset (ms; positive shifts cues later)** to align the transcript with the audio; offsets accept whole milliseconds within ±100 hours. Selecting a cue seeks to `cue start + offset`, clamped at audio time zero, and playback highlighting compares against `audio time - offset`. **Save project…** writes a version 3 workspace project when audio is linked or the offset is nonzero; otherwise it keeps the version 1 transcript format. Version 1 and 2 projects load with a zero offset. Loading a project does not decode linked audio automatically: use **Load linked audio** to open it. Audio paths are validated as relative, and non-portable cross-volume or non-Unicode path relationships are rejected when saving. Transcript input is limited to 16 MiB and 20,000 cues.

- `open-vtt`: Imports simple UTF-8 cues. Cue settings, regions, styles, and timed markup are outside this route. Cue identifiers and NOTE blocks are omitted.
- `open-lrc`: Imports UTF-8 LRC metadata, signed `[offset:milliseconds]`, timestamps such as `[01:23.45]`, and repeated timestamps on one lyric line. Cue ends are inferred from the next timestamp; the final line receives a 3-second duration. Review timings before using the draft.
- `prepare-wav`: Converts mono or stereo PCM16 WAV at 16, 32, or 48 kHz to a new 16 kHz mono WAV. Downsampling averages groups of input frames.
- `audio-info`: 16 kHz mono signed PCM16 WAV.
- `play-wav`: The native audio workspace opens and plays local WAV, FLAC, MP3, OGG/Vorbis, AAC/M4A and AIFF files with 1–8 channels, 8–192 kHz, up to ten minutes, 128 MiB input and 256 MiB decoded memory. Select `Play local audio`, browse to a file, then Run. Use the waveform or seek control to navigate; selecting a transcript cue seeks to its start. The output path and parameter are unused. Cancel stops playback. Decoding is offline; device availability depends on the host audio system.
- `detect-voice-activity`: Reads a 16 kHz mono PCM16 WAV and writes a new JSON report of energy-active intervals. This short-time RMS detector is an estimate. Tones, music and noise may count as activity; quiet or unvoiced speech may be missed. It is not a neural speech detector and does not identify speakers.
- `analyze-transcript`: Reads the SRT input or current draft and writes a new JSON report with an estimated language, classifier confidence, Unicode word/character/grapheme counts, sentences, non-empty text lines, unique-word ratio, cue-covered duration and an estimated words-per-cue-minute rate. The language confidence belongs to the local text classifier, not speech recognition. No ASR confidence is reported because the current decoder does not provide one. Rate estimates use subtitle cue timing, not acoustic speech duration.
- `open-srt`: UTF-8 SRT subset; editable draft.
- `replace-text`: Enter JSON such as `{"find":"old","replace":"new"}`. The operation uses the current draft, or a supplied SRT if the draft is empty, and returns an edited draft. Matches are case-sensitive and confined to individual cues. Timings are preserved. Save explicitly to export the changes.
- `export-srt`: Validated SRT output.
- `export-vtt` writes WebVTT from SRT input or an edited draft. Cue text is exported as literal text.
- `export-lrc` writes timestamped lyric lines with centisecond start times. LRC does not encode cue end times, so timing duration is lost; multiline cue text is joined with spaces.
- `transcribe`: For an ASR-enabled build, leave the parameter blank for the private English Tiny model pack, or enter `multilingual` for the private multilingual Tiny pack.
- `translate-to-english`: For an ASR-enabled build, enter `multilingual` and use the private multilingual Tiny pack to translate recognized speech audio into English cue text. This is Whisper audio translation, not general text translation or translation between arbitrary language pairs. Only the multilingual Whisper Tiny model is accepted. In the audio workspace, select Multilingual Tiny and choose **Translate speech to English**. Review output; language coverage and translation quality have not been measured. See [offline model packs](WHISPER_MODEL_PACK.md) for checksums and setup. The GitHub [offline ASR model-pack workflow](https://github.com/ahmad-v/speech-studio/actions/runs/46127520782) passed and uploaded both expansion packs as `speech-studio-offline-asr-model-packs` (155,402,407 bytes combined, 14-day retention). Those packs are separate from the model-free default application distribution.
- `export-json`: Save transcript text and timings from the SRT path or draft. The schema is `{"schema_version":1,"segments":[{"start_ms":0,"end_ms":1000,"text":"example"}]}`.
- `open-json`: Restore a version 1 transcript into an editable SRT draft. Unsupported versions, unknown fields, invalid timing and text outside the SRT draft subset are rejected.
- `export-txt`: Plain transcript text.

Limitations: Private ASR model packs are separate from the default application build; no model is committed or downloaded by the app. The optional inference source requires an ASR-enabled build and a pinned local pack or explicit model path plus SHA-256. A user-supplied hash is integrity checking, not authenticity. The five-target optional ASR/audio compile and automated test matrix passed at code head `d74b7a8` (see [verification evidence](VERIFICATION.md)); it does not establish recognition accuracy. Multilingual mode asks Whisper to detect language, but language-level quality, WER and CER have not been measured. Runtime CPU memory and speed have not been measured. Cancellation cannot interrupt inference itself. Physical playback and recording, translation, diarization and TTS remain unverified. Decoder fixture checks cover only formats listed in the status report. The bundled energy VAD is not neural and cannot distinguish speech from other sustained sounds. Transcript language identification and linguistic counts are estimates, not ground truth. SRT supports strict basic timecodes and sequential indices; overlapping segments are allowed.

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
