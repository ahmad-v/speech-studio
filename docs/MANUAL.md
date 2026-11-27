# Offline manual | Speech Studio

Development source only. Launch the GUI after compiling. Choose an operation, enter local input path and a new output path where needed, set the parameter as described under the operation selector, then Run. Status and errors are shown below. Cancel requests cooperative cancellation. The draft editor accepts transcript or plain text. Choose a theme to change workbench colors and control borders. UI scale adjusts the controls from 80% to 300% and remembers your appearance choices when you reopen the app. Paths are manually typed in this increment; native dialogs and full accessibility remain pending.

- `open-vtt`: Imports simple UTF-8 cues. Cue settings, regions, styles, and timed markup are outside this route. Cue identifiers and NOTE blocks are omitted.
- `open-lrc`: Imports UTF-8 LRC metadata, signed `[offset:milliseconds]`, timestamps such as `[01:23.45]`, and repeated timestamps on one lyric line. Cue ends are inferred from the next timestamp; the final line receives a 3-second duration. Review timings before using the draft.
- `prepare-wav`: Converts mono or stereo PCM16 WAV at 16, 32, or 48 kHz to a new 16 kHz mono WAV. Downsampling averages groups of input frames.
- `audio-info`: 16 kHz mono signed PCM16 WAV.
- `open-srt`: UTF-8 SRT subset; editable draft.
- `replace-text`: Enter JSON such as `{"find":"old","replace":"new"}`. The operation uses the current draft, or a supplied SRT if the draft is empty, and returns an edited draft. Matches are case-sensitive and confined to individual cues. Timings are preserved. Save explicitly to export the changes.
- `export-srt`: Validated SRT output.
- `export-vtt` writes WebVTT from SRT input or an edited draft. Cue text is exported as literal text.
- `export-lrc` writes timestamped lyric lines with centisecond start times. LRC does not encode cue end times, so timing duration is lost; multiline cue text is joined with spaces.
- `transcribe`: For an ASR-enabled build, leave the parameter blank to use the [private English model pack](WHISPER_MODEL_PACK.md).
- `export-json`: Save transcript text and timings from the SRT path or draft. The schema is `{"schema_version":1,"segments":[{"start_ms":0,"end_ms":1000,"text":"example"}]}`.
- `open-json`: Restore a version 1 transcript into an editable SRT draft. Unsupported versions, unknown fields, invalid timing and text outside the SRT draft subset are rejected.
- `export-txt`: Plain transcript text.

Limitations: No redistributed ASR model or reference speech. Optional native inference source only, not enabled in default build. Private Tiny English pack or explicit local model path with expected SHA-256 required; user hash is integrity, not authenticity. Cancellation cannot interrupt inference itself. No playback, resampling, waveform, language accuracy, WER, translation, diarization, TTS or model pack. SRT supports strict basic timecodes and sequential indices; overlapping segments allowed.

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

The five cue edits return an SRT draft. You can also supply a new .srt output path to save the edit immediately.
