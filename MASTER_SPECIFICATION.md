# Native Offline Studios

Master project specification and development handbook | Version 1.3 | 26 November 2026

Status: approved direction for implementation, not a claim of completed software. Localwright is the approved name of the independent shared library; the four studio names remain working names. This document controls the five repositories and supersedes earlier brainstorming. MUST means a release requirement; TARGET means planned scope to implement and verify; DEFERRED means do not implement now. No feature is supported merely because it appears here.

## 01 | Motivation and public commitments

### Why I started this project

I am a PhD student, and like many PhD students, I do not have much money to spend on software.

I often need tools for analysing data, working with documents, managing files and transcribing recordings. There are many excellent applications, but some are expensive, some require subscriptions, and others depend on cloud services or complicated installations. I wanted to make tools that I could use in my own academic work without having to worry about those things.

I also miss the old days of desktop software. Applications had their own personalities. You could change their appearance, choose your colours, customize toolbars and work with your files without being asked to sign in. I would love to bring some of that experience back.

I want these applications to work locally, to respect people's privacy and to support as many useful file formats as possible. I want them to be available on different operating systems, not just the one I happen to use.

I am starting this project partly because I need these tools myself, but I also want to share them for the public good. I hope they will help students, researchers and anyone who cannot afford expensive software or simply prefers to work offline.

There will be no advertisements, telemetry, mandatory accounts, subscriptions or premium features. Everyone will have the same functionality. If you find the software useful, please consider citing it in your academic work. If you would like to help with code, testing, translations or documentation, you are very welcome. Voluntary donations are appreciated, but never expected.

I want to make useful software, share it and give something back to the open-source community.

### Non-negotiable commitments

- Free, open-source, local and offline after installation, including all advertised capabilities of each edition. No activation, ads, analytics, crash-report uploads, tracking, mandatory account, subscription, paywall or artificial quota.
- Native desktop GUI using Rust and Iced. No Electron, Chromium, WebView/WebView2, Tauri, MERN/MEAN, JavaScript frontend or local HTTP server for the UI. Web formats and project websites are permitted; GUI rendering must not depend on them.
- All processing is local; no hidden network calls, update checks or model downloads. Explicitly selected external links may open the system browser. No embedded payment widgets. Nothing is sent without a deliberate user action.
- Rust first, Python second where useful, mature native libraries and private executables when justified. Minimum necessary dependencies, no system Python or PATH changes, no compulsory external engine installation.
- Preserve original data by default; accessible, customizable and documented interfaces; interoperability and reproducibility; voluntary citation and donations only.

## 02 | Portfolio and boundaries

Five public repositories under the maintainer's GitHub account or organization. GitHub is the canonical upstream for development, CI, releases and issue tracking. After the first verified releases, create five corresponding public GitLab mirrors with maintainer approval. No GitLab infrastructure or subscription is required for initial development. Keep unrelated private repositories outside this project and outside its authorized Codex access. Create all five at the start; one independent installable product per studio, one common source-level library ecosystem. No separate mandatory shared runtime, no standalone media suite, no iPad/iOS, Android, Intel Mac or 32-bit work at present. A future mobile decision requires genuine interest, support and funding; do not spend CI or design effort on it now.

| Repository (provisional) | Purpose | First verified vertical slice |
|---|---|---|
| localwright | Independent modular Rust toolkit, UI and reusable infrastructure | Themes, settings, jobs, format/engine interfaces, tested example |
| file-studio | File inspection, privacy, cataloguing and preservation | Inspect file, hash, export report, scan directory |
| data-studio | Data interoperability, cleaning, statistics and figures | Import CSV/XLSX, preview, filter, summarize, export CSV/Parquet |
| document-studio | Document interchange, PDF, OCR, comparison and publishing | Markdown to DOCX, PDF text extraction and merge/split |
| speech-studio | Offline transcription, annotation, translation and synthesis | Import audio, transcribe with one local model, edit, export SRT/TXT |

All five must be established in a coordinated milestone. Initial slices must be end-to-end real functionality, not mock engines. Expand incrementally across all four products rather than releasing empty shells or requiring feature parity immediately.

### Functional ownership

File Studio owns file identification, generic metadata, archives, privacy, integrity, offline disk catalogues, duplicate analysis, encryption and later backup. Data Studio owns tables, transformation, statistical analysis, figures and scientific data. Document Studio owns structured documents, layout conversion, PDFs, OCR, citations, e-books, document archive and comparison. Speech Studio owns speech recognition, synthesis, translation of speech and text, audio alignment and linguistic formats. Common operations are reusable through narrow interfaces; avoid building four separate implementations of the same export. Audio/video decoding is an enabling capability, not a standalone media editor. Workflow automation grows within each studio and can use a common declarative operation model later.

### Scope decisions still to validate

Brand and repository owner, maintainer name/ORCID, application identifiers, logo artwork, donation handles, minimum OS versions, dependency releases, final license, model choices, package size budgets and actual format capabilities are not yet fixed. Use explicit placeholders only in documentation/templates; never fabricate real identifiers, citations, release results or security claims. Propose and record decisions before locking them. Default candidate license for original Rust code: MIT OR Apache-2.0, subject to dependency review and maintainer approval. Localwright is the approved library identity; the repository slug is localwright. Verify repository and crate-name availability and potential trademark conflicts before publication; report any collision rather than silently renaming. Other names remain provisional until checked.

## 03 | Localwright: independent shared library architecture

Use a Cargo workspace inside the localwright repository. Each app has its own workspace or crate graph, lockfile, versioned release and installer. For local integration use a sibling checkout or temporary Cargo patch; published builds must use an exact immutable shared-library commit or audited released crate versions, not a floating branch. CI verifies all consumers against the intended shared release. No separate runtime install. Prefer small crates, feature flags, minimal defaults, explicit dependency selection and stable public interfaces.

| Module | Responsibility |
|---|---|
| localwright-core | Errors, domain types, IDs, operation contracts |
| localwright-ui / localwright-widgets | Iced shell, menu actions, toolbars, tabs, virtualized tables, dialogs, inspectors |
| localwright-themes / localwright-icons | Semantic style tokens, built-in themes, original/licensed icon resources |
| localwright-layouts / localwright-shortcuts | Layout persistence, command bindings, keyboard focus |
| localwright-palettes / localwright-figures | Palette registry, scientific styles and export interfaces |
| localwright-jobs | Async job queue, progress, cancellation, bounded concurrency |
| localwright-files / localwright-formats | Safe paths, detection, capability registry and import/export routing |
| localwright-storage | Settings and versioned local project persistence; SQLite only when warranted |
| localwright-runtime / localwright-process / localwright-python | Private engine discovery, controlled child processes, protocol and health checks |
| localwright-security / localwright-platform | Input limits, OS integrations, safe file operations |
| localwright-i18n / localwright-accessibility | Localized resources, semantic labels and keyboard navigation |
| localwright-export / localwright-testing | Reusable export contracts, reference fixtures and test helpers |

Names are proposed boundaries, not a mandate to create many empty crates. Start with a few working modules; extract reusable code when there are real consumers. UI code must not own domain algorithms. Heavy backends stay in the relevant app, including LibreOffice, Polars/DuckDB, Python packages, FFmpeg, OCR and ASR models. Shared lightweight adapters can exist without linking the backend. No application may transitively ship an unrelated engine.

### Independent project identity and public API

Localwright is a general-purpose, modular Rust toolkit for native, cross-platform and offline desktop applications. Its proposed academic title is “Localwright: A Modular Rust Toolkit for Native, Cross-Platform and Offline Desktop Applications”. The four studios are initial consumers, not the scope or identity of the library. Use Localwright in prose, `localwright` for the repository, directory and source archive (`localwright.zip`), and `localwright-*` for individual Cargo packages (Rust imports use underscores). Use these names consistently in manifests, dependency references, CI, documentation, examples, test fixtures, citation metadata, handoff status, and the eventual GitLab mirror. Do not retain obsolete shared-project identifiers or create an additional repository for the library.

Keep the dependency direction from applications to Localwright, never from Localwright to a studio. Generic non-GUI crates must not require Iced; optional UI, visualization, Python and platform integration must remain independently selectable. Design public APIs and examples for external developers, with documented features, supported targets, minimum supported Rust version, semantic versioning, compatibility policy, migration notes, API documentation, integration examples and independent tests. Include at least one minimal external-consumer example that does not depend on studio code. Avoid assumptions about studio branding, donation accounts, project file formats or app-specific processing. Each reusable crate should have appropriate package metadata and a public license compatible with its dependencies. Preserve app-specific heavyweight engines in their own applications.

Maintain Localwright's own README, changelog, release tags, contributor guidance and CITATION.cff; archive it separately from the four studios under its own DOI when verified and approved. Prepare individual stable crates for eventual publication to crates.io and documentation on docs.rs; check actual name availability and package contents, publish in dependency order only with maintainer approval and verified credentials. Do not reserve names with empty packages, assume names are available, or make registry publication a prerequisite for initial integration using pinned Git revisions. Do not publish any crate or create remote resources without approval.

### Contracts and data flow

Each engine exposes an ID, version, platform availability, license reference, supported operations, accepted input/output formats, capability limitations, resource requirements and health check. Jobs have unique IDs, typed parameters, progress (indeterminate when unavailable), cancellation and structured results/errors. Use explicit schema versions, serde and testable pure domain functions. Use JSON Lines over stdin/stdout for private workers; binary data, Arrow IPC or private temporary files for large payloads. Logs go to stderr or local files, never the protocol stream. No local network service. Handle worker crash, timeout, invalid response and cancellation without freezing the GUI.

Project files use documented, versioned schemas. Store user assets by explicit paths or managed copies; no hidden file migration or destructive overwrite. Save atomically, support migration and restore, and preserve provenance. Use a shared command registry for menus, shortcuts and toolbars so functionality stays consistent. Cross-project changes must document API impact, tests and migration steps.

## 04 | File Studio

TARGET: a consumer-friendly native file workstation, not a clone of a duplicate finder. Default to read-only inspection and copy-on-write sanitation. No automatic destructive actions.

| Module | Planned capabilities and candidate engines |
|---|---|
| Inspect | Signature detection (infer/libmagic), MIME, timestamps, sizes, encoding, hex viewer, embedded objects, text extraction, format-specific parsers |
| Metadata/privacy | ExifTool, image and Office XML parsers, MediaInfo/ffprobe only where needed; inspect, export, edit and remove supported tags; before/after audit |
| Catalogue | Rust filesystem traversal, SQLite/FTS, stable volume identity where available, offline search of disconnected drives, scan history, QR labels |
| Integrity | BLAKE3, SHA-256/512; MD5/SFV only for legacy verification; versioned manifests, corruption detection, PAR2 generation/recovery |
| Organize | Duplicate detection via hashing, directory usage, regex/metadata batch renaming, previews and undo where safe |
| Archives | ZIP/libarchive/7-Zip as justified; browse, test, extract, create supported formats; enforce path and size limits |
| Encryption/signatures | age and established signature libraries; safe key handling, authenticated encryption; no custom cryptography |
| Later | Snapshot/versioning, optional incremental backups, verified restoration; defer raw disk recovery/diagnostics |

### Format targets (operation-specific)

Identify any file supported by tested detectors; inspect archives ZIP, 7Z, RAR, TAR, CAB, CPIO, AR and compression GZ, BZ2, XZ, ZST, LZ4, LZMA; disk images ISO, IMG, DMG, VHD/VHDX/VMDK (inspection only if safe). Images JPEG, PNG, TIFF, HEIC, AVIF, WebP, JXL and camera RAW DNG/CR2/CR3/NEF/ARW/RAF/ORF/RW2. Graphics PSD/XCF/SVG/AI/EPS; Office and PDF/EPUB; audio WAV/MP3/FLAC/OGG/AAC/AIFF; video MP4/MKV/MOV/AVI/WebM/MPEG; fonts TTF/OTF/WOFF/WOFF2/TTC; executable EXE/DLL/ELF/Mach-O/WASM and MSI/DEB/RPM/APK/JAR as read-only; email EML/MSG/MBOX/PST/OST; DICOM and PCAP/PCAPNG if justified. Metadata EXIF/IPTC/XMP/ICC/ID3. Import/export checksum manifests, PAR2, AGE and supported signature formats. Reports TXT/CSV/JSON/PDF/DOCX/HTML where supported. Never execute inspected files.

### Verification and safety

First slice: identify known fixture formats, compute known hash vectors, scan a test directory and export valid JSON/CSV. Test unreadable files, symlinks, concurrent modification, non-ASCII paths, disconnection and cancellation. Verify cleansed output independently; metadata removal is NOT permanent redaction. Clearly disclose residual metadata, format limitations, and irreversible operations. Never delete duplicates or overwrite originals without explicit preview/confirmation. Archive extraction must reject traversal, unsafe links and expansion bombs. Flatpak edition uses portal access where possible and truthfully reports restricted disk operations.

## 05 | Data Studio

TARGET: a no-code, reproducible data transformation and statistical workstation for research and ordinary users. A native virtualized table, schema inspector, transformation history and chart/analysis workspace are core; no web table or notebook dependency. Rust Polars is a candidate primary DataFrame engine; DuckDB for distinct local SQL workflows, not needless duplication. Use Arrow interchange where appropriate; use Python only for genuinely useful methods or format bridges.

| Module | Planned capabilities and candidate engines |
|---|---|
| Import/preview | Streaming preview, encoding/delimiter/schema detection, worksheet selection, missing values, locale and timezone handling |
| Transform | Filter, sort, deduplicate, join, aggregate, type conversion, recode, pivot/unpivot, split/merge columns, formulas, regex, missing data policies |
| Statistics | Descriptives, distributions, correlation, tests, regression, effect sizes and confidence intervals; later survival/econometrics/ML via vetted SciPy/statsmodels/etc. |
| Analysis | Embedded local SQL, transparent methods and assumptions, optional scientifically justified modeling, random seeds |
| Visualize | Histograms, bars, scatter, lines, box/violin, heatmaps, facets, error bars, accessible legends; vector/raster export |
| Projects | Reusable declarative pipelines, provenance, versioned transformations, undo/redo, batch runs, local project files |
| Privacy | Rule-based masking/pseudonymization, preview and warnings; no claim of guaranteed anonymization |

### Format targets

CSV, TSV, PSV, fixed-width/DAT; XLS/XLSX/XLSM/XLSB/ODS; JSON/JSONL/NDJSON, XML/YAML/TOML; Parquet/Feather/Arrow IPC/ORC; Avro/MessagePack/BSON; SPSS SAV/ZSAV/POR; Stata DTA; SAS SAS7BDAT/SAS7BCAT/XPT; R RDS/RData; MATLAB MAT; NumPy NPY/NPZ; HDF5/HDF4/NetCDF/Zarr; FITS/VOTable; SQLite/DuckDB/DBF/MDB/ACCDB; GeoJSON/SHP/GPKG/KML/GPX; SDMX/DDI; GraphML/GEXF/GML. Scientific, statistical, geospatial and graph formats are later targets, not necessarily table-compatible or writable. Export reports in PDF/DOCX/HTML/Markdown and figures in supported vector/raster formats. Provide explicit selections for nested/multidimensional data and do not silently flatten structures.

Candidates: csv, calamine, rust_xlsxwriter, serde_json, quick-xml, Arrow, PyReadStat (statistical formats; verify per-format write support), pyreadr, SciPy, h5py, Xarray, GDAL and specialized libraries only when necessary. No compulsory full Python scientific stack for basic CSV work. Keep common data and scientific packs application-local.

### Verification

Reference fixtures for types, missing values, Unicode, encoding, joins, statistical labels, dates, formulas and round-trip preservation. Compare numerical results against published reference datasets or independently implemented trusted tools; state tolerances, degrees of freedom, assumptions, sample sizes, missing-data rules and multiple-testing corrections when applicable. No fabricated significance or unsupported tests. Test large inputs, virtualized rendering, bounded memory and supported out-of-core execution; never promise every operation works beyond RAM. Report conversion loss and spreadsheet-formula injection risk, and provide safe export options.

## 06 | Document Studio

TARGET: a document interoperability workstation with multiple conversion routes, native preview and clear limitations. Distinguish content transformation, layout-preserving rendering, text extraction and structural manipulation. Do not promise perfect Office fidelity.

| Module | Planned capabilities and candidate engines |
|---|---|
| Conversion | Pandoc for structured markup/office/e-books; native Rust for simpler conversions; private specialized filters for legacy formats |
| PDF | PDFium for rendering, qpdf/lopdf for structural operations, pypdf/pikepdf only if justified; merge/split/reorder/extract/compress/annotate, later validated permanent redaction |
| OCR | Tesseract and bundled language data; optional lightweight layout recognition; searchable PDF with independently tested text layer |
| Publishing | Typst, templates, academic reports, citations, bibliography conversion, preview; avoid full office editor scope |
| E-books | EPUB parsing/validation, optional Calibre conversion components only if smaller tools insufficient; no DRM circumvention |
| Compare/archive | Text and structure diffs, page comparisons, local document indexing and search, metadata and citation management |

### Format targets

DOC/DOCX/DOCM, RTF, ODT/OTT/ODS/ODP, PPT/PPTX, TXT (UTF-8/16 and legacy encodings), PDF/PDF-A/PDF-X only with appropriate validation, Markdown/CommonMark/GFM, TeX/LaTeX, Typst, JATS, RST, AsciiDoc, DocBook, DITA, HTML/XHTML/MHTML, EPUB/MOBI/AZW3/FB2, WPD/WPS/SXW, Apple iWork and historical Macintosh formats where filters exist, BibTeX/BibLaTeX/RIS/ENW/CSL JSON, DjVu/TIFF/PNG/JPEG, SVG/EPS/PS, EML/MSG/MBOX. Targets are directional, not all-to-all conversion. HTML rendering uses safe native structured preview or offline conversion, never WebView; disable remote resources and scripts.

### LibreOffice policy

Use only when simpler engines cannot provide adequate Office layout or legacy conversion. Prefer individual libwpd/libwps/libetonyek/libmwaw/librevenge/etc. where usable. If necessary package a minimal tested private headless LibreOffice distribution, retaining required core, filters, Writer/Calc/Impress and rendering resources; no Base/help/examples/templates/dictionaries/Java unless necessary. Do not delete internal dependencies blindly. Do not expose splash, UI, console or desktop entries; no file associations or system installation. Use absolute path and a unique private profile (UserInstallation). Keep it exclusively in Document Studio, never a required shared crate or other studio dependency. Document licensing, attribution and measured package size. Avoid building LibreOffice from source until a real need justifies effort; if minimum package is not yet feasible, omit that route and disclose it.

### Verification

Fixtures for DOCX to PDF/Markdown, Markdown to DOCX, PDF merge/split/extract and scanned OCR. Validate generated files with independent readers; compare page count, text, fonts, tables, layout and visual snapshots as applicable. For redaction verify that hidden text, annotations, attachments and content cannot be recovered; otherwise do not advertise redaction. Untrusted files/macros never execute. Use private font resources and report missing fonts and substitutions.

## 07 | Speech Studio

TARGET: a native offline multilingual speech and linguistic workstation. No cloud AI. CPU-first, optional tested hardware acceleration. Rust for audio pipeline, UI and transcript operations; prefer linked whisper.cpp/whisper-rs or Sherpa-ONNX/ONNX Runtime and compact models over full training frameworks; Python only for missing capabilities.

| Module | Planned capabilities |
|---|---|
| Transcribe | Audio/video import, recording, one verified local ASR model initially, timestamps, segmentation, batch jobs, multilingual model selection; later Whisper Tiny/Base and compatible Moonshine/Zipformer |
| Edit | Native waveform/timeline, playback, editable segments, speaker labels, find/replace, subtitle timing, undo, project save |
| Analyze | VAD, optional diarization, language detection and linguistic annotation, confidence when actually provided by model |
| Translate | Local text/transcript translation via vetted Marian/OPUS/Argos or linked inference equivalents; preserve source and time alignment |
| Narrate | Offline TTS using vetted Kokoro/Piper or equivalent, pronunciation and voice controls, chaptered audiobook exports |
| Enhance | Optional noise reduction, resampling and speech segmentation only as required; no general audio editor |

Candidates: Symphonia, FFmpeg (private executable or linked components where necessary), CPAL, Rubato, Rodio or platform adapters, Silero VAD, native subtitle formats, SQLite where useful. Models and voices have separate licenses; no automatic model downloads. Bundle at least one suitable redistributable CPU model for advertised transcription; optional offline packages for larger models. Record checkpoint, version/hash, supported languages, quantization, inference backend and resource requirements. Do not claim diarization or confidence scores if engine cannot supply them.

### Import and export targets

Audio WAV/AIFF/PCM/AU, FLAC/ALAC/APE/WavPack, MP3/AAC/M4A/OGG/Opus/WMA, AMR/3GP/CAF/M4B; video MP4/MKV/AVI/MOV/WebM/MPEG/FLV/WMV/VOB; subtitle SRT/VTT/ASS/SSA/TTML/SBV/SAMI/LRC; transcripts TXT/MD/RTF/DOCX/ODT/PDF/HTML/JSON/JSONL/XML/CSV/TSV/XLSX/Parquet; academic annotation ELAN EAF/Praat TextGrid/CHAT/TRS; translation exchange XLIFF/TMX; synthesized audio WAV/FLAC/MP3/OGG/Opus and M4B/chaptered audio; EPUB and academic report exports where supported. Do not infer transcription support for all codec/container combinations; capability registry must list verified decoding and encoding separately.

### Verification

Use licensed short reference recordings, silence, multiple languages, noise, invalid audio and long-duration fixtures. Measure WER/CER and timestamp tolerance on labeled fixtures, document limitations, verify no hallucinated output on silence where possible, ensure cancellation and responsive UI. Check transcript round trips, subtitle timing validity, audio normalization and exported file playback. Show estimates/uncertainty as such; no fabricated speaker identities. Provide model and method citations in reproducibility reports, with user control over inclusion.

## 08 | Native interface, themes and customization

Common Iced desktop shell: OS window decorations by default, traditional menu bar (File, Edit, View, Project, studio-specific menu, Tools, Window, Help), customizable toolbars, tabbed projects, navigable sidebar, resizable panels, properties inspector, status bar, local task manager, context menus, drag/drop, native file dialogs, configurable shortcuts, optional recent files, full-screen/distraction-free views, and offline documentation. Multiple windows and detachable panels where tested. No forced home dashboard, oversized controls or needless whitespace. Keep expensive work off UI thread; virtualize large views. Follow platform keyboard and window conventions. System-native title bars may not accept arbitrary theme styles; theme the client area, do not sacrifice OS accessibility to fake title bars.

### Twenty built-in themes

| Family | Themes |
|---|---|
| Modern | Modern Light, Modern Dark, Graphite, Midnight |
| Historical | Classic 95, Classic 2000, Luna, Silver, Aqua, Workstation, Retro Beige |
| Academic | Paper, Nordic, Solarized Light, Solarized Dark |
| Monochrome/accessibility | Phosphor Green, Amber Terminal, Monochrome, High Contrast Light, High Contrast Dark |

Original historical interpretations, not copied proprietary assets. Use verified licensed assets and consider distinct public-facing names to avoid trademark confusion. Themes change widgets as well as colors: square/round borders, raised/beveled/flat/glossy styles, selection, scrollbars, menu and table styling. Iced custom widgets/rendering are needed for some effects; no promise until validated. Default: system light/dark preference with an easy theme selector; no forced retro. System appearance is a mode, not an extra named preset.

### UI settings

Independent theme, density (compact/standard/comfortable/large), scaling (automatic or 80–300% where usable), font and monospace font, font size, accent, corner radius, borders, shadows, gradients, optional transparency and reduced motion. Configurable icon size, icons/text/both, tooltips, scrollbar width, gridlines, alternating rows, sidebar left/right, tab placement, visible panels and status bar. Four initial icon styles: modern outline, classic colourful, monochrome and original/licensed retro pixel. Four workspace presets: classic desktop, modern workspace, professional workstation, distraction-free. Save/import/export layouts and shortcuts. Live settings preview, cancel, reset and safe fallback if invalid. Local TOML/JSON declarative themes, icon packs, palettes, layouts and presets; no executable code, remote resources or unsafe SVG. Schema-versioned, validated and bounded. User fonts may be selected locally; bundle only licensed fonts needed for rendering.

### Accessibility

Keyboard navigation, visible focus, semantic labels, high contrast, scalable text, reduced motion, non-color status indicators, Unicode, localization and RTL target. Test Iced screen-reader integration on all supported OSs; report actual gaps, do not claim full accessibility on the basis of theme names. Native menu, dialogs, print, clipboard and drag-drop should be tested. Restore safe defaults after invalid theme/layout imports. Help menu contains offline manual, shortcuts, format registry, citation, scientific references, source, bug report, contribute, support, licenses, about. External links open only upon explicit click.

## 09 | Scientific palettes and figures

Separate UI themes from document, chart, waveform, heatmap and report styling. Store palette selection and figure settings in project/figure metadata. Default scientific sequential palette Viridis or Cividis; categorical Okabe-Ito; diverging palette with justified midpoint; black-and-white option. Do not apply UI dark mode to exported figures automatically.

| Family | Initial and expansion palette targets |
|---|---|
| Sequential | Viridis, Magma, Inferno, Plasma, Cividis, Batlow, Grayscale, Blues, Greens |
| Diverging | Coolwarm, Red-Blue, Purple-Green, Brown-Teal; additional scientific palettes |
| Categorical | Okabe-Ito, Tableau 10, ColorBrewer Set1/Set2/Dark2, traditional colors |
| Cyclic and legacy | Appropriate cyclic maps, terrain, heatmaps and traditional rainbow/Jet for compatibility, not default |
| Monochrome | Grayscale ramps, pure black/white, print-safe patterns and distinct line/marker combinations |

Use authoritative palette values rather than hand-recreated approximate previews; record sources, licensing and attribution. Include Crameri scientific maps where permitted. Support reversal, custom stops/categories, clipping, explicit missing-value style, normalization and logarithmic axes where mathematically appropriate. Do not conflate sequential and categorical maps. Preview protanopia, deuteranopia, tritanopia and grayscale; a preview is not a guarantee of accessibility. Use redundant encodings: labels, line styles, markers, hatching, patterns, contrast and meaningful legends. Check printed readability; warn on low contrast or too many categories.

### Publication settings and exports

Custom dimensions in pixels/mm/cm/inches, journal presets, 72/150/300/600/1200 DPI and custom for raster output; vector exports are resolution independent except rasterized layers. Font families and sizes, axes/ticks, units, legends, grids, line widths, marker shapes, confidence intervals/error bars, background transparent/white/color, multi-panel layouts and reusable templates. Target SVG/PDF/EPS/PS and EMF where feasible; PNG/TIFF/JPEG/WebP/BMP as justified. Do not label screenshot-based PDF or SVG as true vector output. Embed/subset fonts where permitted, or document substitutions. Preview final export at target print size. File Studio and Document Studio reports use relevant chart/report features without pulling a heavyweight statistics engine; Speech Studio uses consistent waveform/spectrogram styling. Data exports must not change underlying values because of display color choices. No forced watermarks, branding or citations on user-generated files.

## 10 | Format registry and private runtimes

The format lists in sections 04–07 are development targets, not verified support. Build a machine-readable registry of format ID, extensions, MIME/signature, versions, direction (identify, inspect, import, export, edit, convert), supported operations, engine/version, OS/architecture, dependency pack, known losses and validation fixture IDs. Show only available routes in dialogs; unsupported items can appear in a clearly labeled roadmap. Distinguish file extensions from codec/format variants. Conversion routing must favor verified direct routes and warn before lossy intermediate conversions. Never claim all-to-all support.

Import/export dialogs: input detection and overrides, output type, filename/location, presets, format-specific settings, metadata policy, encoding/locale, validation, original preservation, collision policy and batch queue. Provide a report of success, failure, warnings and conversion losses. Test content and metadata round trips where meaningful; validate with independent readers. Support legacy data access when safe, not unrestricted execution of legacy content. Documentation and compatibility registry must be generated or reconciled with actual tested capabilities per platform.

### Runtime rules

Each studio bundles its own required engine binaries, private native libraries, Python runtime/packages if needed, model/language/font assets and versioned manifests. No separately installed Pandoc, LibreOffice, FFmpeg, ExifTool, Python, pip, Java or Rust. No global/user PATH modifications, system Python modifications, system package-manager calls, environment pollution or writes to another app's configuration. Normal OS libraries/drivers and Flatpak runtime are explicit platform exceptions. No administrative rights for normal processing. Each installer is independent and uninstalling one must not break another.

Use application-relative discovery resolved to absolute trusted paths; no shell interpolation or ambient executable fallback. Control inherited environment, process-local PATH, library search paths and working directory. Keep necessary OS variables and remove risky PYTHONPATH/PYTHONHOME/user-site and dynamic-loader overrides where appropriate. Private Python via verified relocatable distribution, pinned wheels or controlled native builds per target, isolated mode (-I) and explicit trusted module entry point. Do not pip install at runtime. Do not assume a copied virtualenv is relocatable. Hide child console windows; manage cancellation, descendant processes and timeouts; preserve engine stderr locally. Use application-specific settings/cache/temp/engine profiles and clean up safely. Portable mode may store settings next to app if writable; never use installed program directory for routine writes.

### Optional packs and supply chain

Offer complete offline installer/package and, where justified, smaller standard package plus offline-installable signed/hashed expansion packs. No payment or feature gating, all packs free. Candidate packs: legacy/office documents, scientific/statistical/geospatial formats, extra OCR languages and speech models. Advertise only included capabilities until pack is installed. No automatic downloads, arbitrary executable plugins or system package manager. Verify package manifest, architecture, integrity and license before import; reject path traversal. Store-specific policies may require a different distribution mechanism. Keep versions, hashes, upstream URLs, licenses, SBOM and resource sizes; download vetted pinned assets in CI. Do not commit large binaries or private data to Git.

## 11 | Security, privacy and reliability

Dependency isolation is not sandboxing. Treat every imported file, archive, metadata field, model and theme as untrusted. Apply bounds to file size, depth, expansion ratio, memory, job concurrency, time and output. Reject path traversal, symlink escapes, device paths, unsafe external references, scripts, macros and executable content. Never run user-supplied commands. Avoid shell invocation; use structured argument arrays and an engine allowlist. Do not follow network references in documents or resolve remote resources. Separate private job scratch directories; clean up without touching unrelated files. Avoid unsafe Rust unless justified and reviewed; FFI wrappers must define ownership, lifetimes and error propagation.

Preserve original files by default; write to a temporary sibling when safe, verify, then atomically replace output where supported. Handle collisions, permission failures, disk-full conditions, concurrent changes, interrupted jobs and cancellation. Explicit confirmation for destructive operations, backup/recovery plans and audit trail. No unsafe automatic deletion, encryption with custom algorithms, or unverified recovery promise. Secrets stay in OS-approved key storage or user-controlled key files with appropriate permissions, not logs, source or project archives. Avoid persistent sensitive metadata unless user chooses it; offer clearing of history, caches and local logs. No diagnostic upload. Redact sensitive paths when users explicitly export logs. Never scan unrelated user directories without consent.

### Dependency and contribution trust

Use pinned toolchains, lockfiles, checksums, audited sources and a dependency inventory. Audit Rust advisories, Python/native dependency vulnerabilities, binary signatures and model licenses; review changes to workflows and build scripts. Generate SBOM (SPDX or CycloneDX), third-party notices and reproducible build instructions. Pin CI actions to reviewed commit SHAs and update deliberately. PR jobs use read-only minimal token permissions, no secrets and no privileged execution of untrusted contributions. Never use pull_request_target to build untrusted PR code with secrets. Release/signing jobs use protected branches/tags and approval, narrow permissions, separate environment and secure secret handling. Keep public logs free of personal data and credentials. Build provenance/attestations where available and free; distinguish them from OS-trusted code signing. Release verification must cover no-network operation and private runtime resolution, not simply a privacy statement.

## 12 | Platforms, packaging and signing

All five desktop compilation targets are in scope from the beginning. Mark an individual target/package experimental or blocked until validated; never silently ship emulated or wrong-architecture dependencies as native. No Intel Mac, 32-bit, iPad or Android. Choose minimum supported Windows/macOS/Linux versions based on toolchain, libraries and actual tests, document them, and pin runner images where practical.

| OS and architecture | Rust target | Initial distribution |
|---|---|---|
| Windows x86-64 | x86_64-pc-windows-msvc | EXE installer, MSI installer, portable ZIP |
| Windows ARM64 | aarch64-pc-windows-msvc | EXE installer, MSI installer, portable ZIP |
| macOS Apple Silicon | aarch64-apple-darwin | DMG with APP bundle |
| Linux x86-64 | x86_64-unknown-linux-gnu | DEB, RPM, AppImage, Flatpak |
| Linux ARM64 | aarch64-unknown-linux-gnu | DEB, RPM, AppImage, Flatpak |

15 package/architecture combinations per studio (60 target combinations total; Flatpak may be distributed through a repository rather than separate files). Use compatible build tooling, candidate cargo-packager, WiX or other audited native installer tooling, cargo-deb/cargo-generate-rpm and AppImage tooling. Build both a standalone offline setup EXE and an MSI installer from the same versioned payload for each Windows architecture; neither replaces the other. The EXE is an installer, not the bare application executable. A local MSI bootstrapper is acceptable if all payloads are embedded and no network is required. Verify native x64/ARM64 compatibility of the installer tooling and all payloads; never assume ARM64 support. Candidate EXE tooling includes a suitable WiX Burn bootstrapper or other audited native installer builder; verify actual support before selection. Portable ZIP contains all advertised private dependencies, no installer. Both installers have optional shortcuts, uninstall support and local app-specific data; no unwanted associations or admin requirement at runtime. Test install, upgrade, repair where supported, uninstall and switching between EXE and MSI without duplicate installs or loss of user projects. Use consistent product identity and versioning. Provide clear offline EXE/MSI download labels. macOS DMG drag-to-Applications; bundle ARM64 helpers/libraries only, ad hoc sign nested components and app in correct order. Clearly label non-notarized downloads, do not recommend disabling Gatekeeper globally. User launch may be blocked by policy. Future Developer ID signing/notarization and Mac App Store releases for all four, subject to sandbox, helper signing, donation and dependency licensing requirements. Keep GitHub releases as well. No developer membership purchase now.

Linux DEB/RPM require distro-specific compatibility testing and normal package-manager installation; app itself never invokes system package managers. AppImage compatibility, FUSE requirements and glibc baseline must be tested. Flatpak from start: app IDs, desktop files, AppStream metadata, licensed icons, pinned runtime/SDK and reproducible manifest with bundled dependencies; no runtime network permission; document portal/sandbox limitations, especially File Studio. Flathub submission is separate, subject to approval and rules; offline runtime may need preinstallation. AUR binary PKGBUILD later after stable releases, source package optional by contributors; Snap not planned. Do not create other repositories without owner approval; Flathub infrastructure may require its own external submission repository later.

Signing: Windows EXE installers, MSI packages and relevant executable payloads initially unsigned if necessary, disclose SmartScreen and trust status; apply for SignPath Foundation after eligible project release, not guaranteed. Microsoft Store MSIX optional later. Linux GPG signed checksums, RPM package signing, DEB signed repo metadata if operating a repo, detached signatures for direct assets, Flatpak repo signing. Publish SHA256SUMS, verified signatures where available, SBOM, licenses, build provenance and release notes. Never claim trust/signing until verified; no private keys or secrets in archives.

## 13 | CI, cost and repository integration

Use free standard GitHub-hosted runners in the five public repositories. Reference the official current runner table before committing labels. Candidate labels: windows-2025 or other supported x64, windows-11-arm, ubuntu-24.04, ubuntu-24.04-arm, and a verified standard Apple Silicon macOS runner. Check actual architecture at runtime; no Intel build. GitHub Actions can build, test and package without local Rust. Free standard runners are subject to job/concurrency/storage/service limits; avoid paid larger runners and add no paid services. Building is not the same as testing actual installed GUI behavior. Human/device validation still needed. Codex reasoning, log reading and fixes still consume allowance, even with remote compilation.

### Reusable workflows and gates

- check: formatting, cargo check --locked, clippy with appropriate warnings policy, and dependency graph for affected packages; quick Linux baseline for development.
- test: unit, integration, format/round-trip, golden/reference, security, runtime isolation and no-network tests. All targets on pull requests or explicit matrix validation; affected-only checks when safe. Shared API changes trigger all consumer checks against pinned revision.
- package: build optimized binaries, assemble only required locked private runtimes and assets, validate architecture and hashes, build 15 target package combinations, separate EXE/MSI install, upgrade and uninstall smoke tests and portable ZIP launch tests, package size and SBOM. Flatpak build and sandbox tests as applicable.
- release: owner-approved tagged release only after required gates, generate checksums/attestations/signatures and metadata. Separate secrets from untrusted builds; no automatic public release from an unreviewed branch. Use draft release or artifacts during validation.
- audit: dependency vulnerabilities, license policy, forbidden runtime dependencies and periodic update reports. Benchmark with documented representative inputs, versions and machine info. No hidden telemetry.

Cache by target/toolchain/dependency lock hash; do not allow untrusted cache poisoning or skip validation after cache hit. Cancel obsolete runs, constrain concurrency and avoid unnecessary repeated full builds. Limit artifacts and retention, particularly model assets and large optional packs. GitHub release size limits may require split packs; verify limits instead of inventing. No continuous polling or autonomous infinite retries. Fetch failed log excerpts and group root causes, target fix, rerun the smallest meaningful check. Default Codex correction budget: up to three focused iterations per failure group before report and maintainer decision; a suggestion for cost control, not a fixed platform limitation. Use local execution when faster or needed for diagnosis. If GitHub API/network access is unavailable, request logs or a maintainer-triggered run rather than pretend it was performed.

### Five-repository contract

Maintainer manually creates five public repositories and grants Codex selected-repository access only. Use a local Codex workspace with five sibling source trees for multi-repository ZIP import if supported, or import source first and use separate authorized repository tasks in Codex Cloud. Do not assume cloud tasks can read local ZIP files, push to other repositories, or operate GitHub Actions; verify capabilities first. Keep repository-specific AGENTS.md and a canonical copy of this specification in each project; propagate changes consistently. Verify actual permission and remote before writing. Use feature/integration branches and pull requests, no force pushes or destructive commands, no changes outside authorized projects. Bootstrap Localwright (localwright) first, commit immutable revision, update app references from local checkout to real pinned revision, then push four app branches and validate against it. Maintain status matrix with commit, CI run URL, target, package, test outcome and blockers. Preserve maintainer control over releases and credentials. Never upload personal academic files or private metadata.

### GitLab resilience mirrors (after first verified releases)

GitHub remains the single writable upstream and canonical citation URL. After owner approval, create five corresponding public GitLab projects under a dedicated public group, and copy all intended public branches, tags and commit history. Use an initial import or standard Git pushes; on GitLab Free, use owner-triggered/manual updates or an explicitly authorized GitHub Actions job with narrowly scoped GitLab credentials rather than depending on paid GitLab pull mirroring. Do not use bidirectional mirroring, blindly force-update branches, delete downstream refs or mirror private projects. Verify refs and commit hashes, default branch, Git LFS objects if any, and repository accessibility. Mark GitLab copies as mirrors in their README/project description and direct contributions to GitHub. Keep GitLab CI disabled for mirrors unless explicitly authorized; GitHub Actions stays primary. Keep mirror failures separate from upstream release gates and report them.

Git history does not contain all release assets, issues, pull requests, wikis, CI logs, LFS objects or hosting settings. Maintain a separate local backup of source history, verified installers, checksums, SBOMs, documentation and archival metadata. When authorized, copy release notes and distributable assets to GitLab release/package storage subject to free storage and size limits; verify SHA256 and disclose any omissions. Never copy credentials or private data. Use the same DOI for the same release on both hosts; no duplicate DOI or automatic second publication. Store credentials only in approved secret storage, never in prompts, repositories or logs; mirror jobs require explicit setup and review. Do not create GitLab mirrors or configure cross-host automation before the maintainer approves the destination projects and first verified release. See https://docs.gitlab.com/user/project/repository/mirror/ and https://docs.gitlab.com/user/project/repository/mirror/pull/ for current plan restrictions.

## 14 | Verification and release acceptance

No simulated functionality, fabricated test results or claims of compatibility without evidence. A green build is necessary but insufficient. All advertised operations require actual implementation and verification. Hidden or clearly labeled experimental modules may be absent until ready. No disabled placeholder controls masquerading as finished features.

| Gate | Required evidence |
|---|---|
| Source integrity | Five coordinated source trees, lockfiles where resolvable, tests, no unresolved internal API mismatch; reproducible checkout |
| Compilation | Actual cargo check/build/test logs on five native desktop targets; status for each target |
| Functional | Each vertical slice works end-to-end; independent expected output; regression fixtures |
| Formats | Directional capability table linked to tests; valid output; documented losses; malformed input handling |
| Isolation | Clean environment without system engines; conflicting system Python/PATH test; absolute private paths; independent uninstall |
| Offline/privacy | Runtime with network disabled; no first-run downloads, outbound calls, telemetry, activation or account |
| Safety | Original preservation, collision and error handling, job cancellation, archive traversal/bomb tests, privacy safeguards |
| UI/accessibility | Real keyboard navigation, focus, font scaling, themes, high contrast, screen-reader tests and platform-specific issues |
| Scientific | Reference numerical results and tolerances; palette verification, grayscale/print readability, valid vector/raster output |
| Performance | Startup, disk size, RAM, large inputs, CPU fallback and measured response; budgets set from evidence, not invented |
| Distribution | Install (where applicable), launch, process fixture, upgrade and uninstall each advertised package/architecture; both Windows installer types and portable ZIP tested separately; correct ABI, signatures and runtime content |
| Documentation | Offline manual, supported format list, changelog, LICENSE/notices, SBOM, citations, limitations, support/contact |

Automated tests can cover much but not substitute for hands-on GUI checks on clean supported operating systems and representative hardware. ARM64 and macOS can be experimental until human validation. Test DEB and RPM on named supported distributions, AppImage in clean distro environment, Flatpak portals and no-network access, Windows EXE and MSI installers separately, including upgrade/uninstall and portable ZIP launch from non-ASCII paths, macOS bundle launch under default security policy or document explicit limitation. Validate screen readers (e.g. NVDA, VoiceOver, Orca where relevant). Review privacy and security manually for destructive features, licensing separately. Release status values: planned, implemented-unverified, CI-tested, device-tested, blocked, released. Store evidence as text/JSON in docs and CI artifacts. No unverifiable claims. If a package fails, withhold it and report missing target; do not block unrelated working packages forever, but clearly distinguish partial/experimental release from full acceptance.

## 15 | Licensing, citations, academic archiving and funding

Original code candidate MIT OR Apache-2.0, subject to maintainer choice and audited dependency licenses. Maintain LICENSE(s), third-party notices, dependency/version/source/license manifest, source availability obligations, model/voice/palette/font/icon terms and SBOM per release. GPL/LGPL tools (e.g. Pandoc, some FFmpeg builds, LibreOffice components) require careful redistribution and integration review; subprocess boundaries alone do not settle obligations. No incompatible or nonredistributable assets, no copied proprietary icons, no invented license conclusions. Review Mac App Store compatibility separately. Clearly identify upstream contributors and actual authorship; do not assign credit without basis. AI-assisted code must be human reviewed and documented transparently; software DOI is not peer review.

### Citation workflow

Each repo includes valid CITATION.cff, matching .zenodo.json if used, optional codemeta.json, AUTHORS and README citation section. Maintainer fills real name, ORCID if available, title, version, date, license, keywords, URL and contributors. Do not insert fake DOIs. If both metadata files are used, Zenodo prioritizes .zenodo.json. Keep them consistent. Help > Cite this software displays actual version DOI once available, concept DOI for general references, offline copy/export BibTeX/RIS/CSL JSON and styles (APA, Harvard, Vancouver, IEEE where validated). Before DOI, cite version and repository URL. Ask users politely to cite software in research and relevant engines/methods/models where appropriate; never force citation, watermark, telemetry or attribution in user output. Optional reproducibility reports record input hashes if chosen, operations, parameters, seed, method, versions, model hashes, citations, and warnings. Sensitive information excluded by default.

Zenodo comes after a first usable, tested milestone and BEFORE publishing the release selected for archival. Maintainer connects GitHub to Zenodo, enables each of the five repos, verifies metadata and release tag, then publishes approved GitHub release. Verify Zenodo record and DOI, actual archived source, license and version. It creates a persistent release snapshot, NOT GitHub repository read-only mode; development, PRs, Actions and subsequent releases continue normally. One concept DOI per project plus version DOIs; cite exact version for reproducibility. Do not assume binaries, external models or submodules are archived automatically; provide stable references/checksums and archive appropriate artifacts separately when rights/size allow. First release may be v0.1.0 if genuinely usable, not a fake v1.0.0. Update README badge and citation metadata with real DOI when issued, record actual snapshot; publish corrected version rather than overwrite archived source. Do not auto-archive every experiment. Verify all five records before announcing them. Zenodo is the preferred archive while GitHub is upstream. Figshare is an optional alternative if the maintainer selects it, not an additional DOI for the same release. For Figshare, use a verified versioned source archive with metadata and checksums, deposit manually or through an authorized API, and record actual DOI/version semantics. Keep citation handling provider-independent, use the same DOI in both repository hosts, and do not require any archive service for application runtime. No archive account or API credentials are required in initial development.

### Support and public good

One optional shared funding page/account set: Ko-fi, GitHub Sponsors, Patreon if desired; no exclusive features, early access or tiers with software privileges. No pop-ups or tracking SDK. GitHub FUNDING.yml uses real handles only after maintainer supplies them. Help > Support, About, README and website link to external browser on user action only. Keep donation links optional; review store-specific rules. Support can help pay for signing, development, hardware, security/accessibility review and maintenance. Nonfinancial contribution equally welcome. Donation proceeds may have tax/reporting obligations; verify local circumstances. No guaranteed funds or publication recognition.

## 16 | Community, repository contents and milestones

Each repository: README, MANIFESTO (same human text), LICENSE and notices, CONTRIBUTING, CODE_OF_CONDUCT, SECURITY (private vulnerability contact when available), AUTHORS, CHANGELOG, CITATION.cff, optional .zenodo.json and codemeta.json, AGENTS.md, docs/ARCHITECTURE, CODING_STANDARDS, FORMAT_COMPATIBILITY, DEPENDENCIES, BUILD, TESTING, ROADMAP and STATUS; include actual examples, screenshots only of running app, accessible offline manual and test fixtures with license/provenance. Use issue templates, good-first-issue/help-wanted labels and clear governance. The Localwright repo also holds reusable CI conventions, theme/palette schema, test utilities and app contracts. No need for sixth repo until explicitly authorized. Website could use GitHub Pages from an existing repo; voluntary later expansion.

| Milestone | Deliverable and exit criteria |
|---|---|
| M0 | Freeze specification, names/owner placeholders, architecture and dependency decision register; create coordinated source generation plan |
| M1 | Generate five complete source ZIPs with working initial vertical slices, shared interfaces, tests, docs and CI; unverified features labeled honestly |
| M2 | Import into five public repos via Codex; pin Localwright revision, run CI, targeted fixes; no public release until approval |
| M3 | Expand all four studios in coordinated feature batches: formats, processing, themes, figures, private runtime; test as each slice grows |
| M4 | Package and validate supported OS/architectures, offline/isolation, security, accessibility, manual QA; mark unsupported combinations |
| M5 | Maintainer enables Zenodo before first selected verified release; publishes approved releases, verifies citations/DOIs and signed checksums |
| M6 | After first verified releases and owner approval, establish five GitLab mirrors, verify refs and separately back up/copy release assets; keep GitHub canonical |
| M7 | Maintain, accept contributions, widen format coverage, update mirrors, add AUR later; consider Apple membership/Mac App Store when support permits |

All five projects start together, but readiness is evidence-based and release versions independent. Do not try to generate all long-term features in one response or publish incomplete shells as finished software. Update a shared feature/target/status matrix every milestone. Adopt semantic versioning (0.x for early releases) and pin shared versions per app. Cost-saving workflow: regular ChatGPT writes code and specs; Codex integrates, reviews, fixes focused defects and uses Actions; user downloads and manually verifies; CI produces packages. If AI chat context runs out, carry forward the exact source snapshot, STATUS, architecture decisions and outstanding work, not a summary alone. Do not assume any background work continues after a chat response.

### Suggested official verification references

Consult current official docs at implementation time rather than guessing versions, runner labels, API signatures, licensing or service policies.

- GitHub runners: https://docs.github.com/en/actions/reference/runners/github-hosted-runners
- GitHub Actions syntax: https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax
- GitLab mirrors: https://docs.gitlab.com/user/project/repository/mirror/
- Figshare API (optional alternative): https://docs.figshare.com/
- Codex Cloud: https://help.openai.com/en/articles/20001545-using-codex-cloud
- Zenodo metadata: https://help.zenodo.org/docs/github/describe-software/
- Zenodo GitHub integration: https://zenodo.org/account/settings/github/
- Apple macOS distribution: https://developer.apple.com/macos/distribution/
- Apple notarization: https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
- Flatpak: https://docs.flatpak.org/en/latest/first-build.html
- Flathub requirements: https://docs.flathub.org/docs/for-app-authors/requirements
- Iced: https://github.com/iced-rs/iced
- Cargo package publication: https://doc.rust-lang.org/cargo/reference/publishing.html
- Crate registry: https://crates.io/
- Python standalone: https://github.com/astral-sh/python-build-standalone
- Citation File Format: https://citation-file-format.github.io/
- SignPath Foundation: https://signpath.org/

External policy links were checked selectively in the previous revision for runner availability, GitLab mirroring, Codex access, Zenodo metadata, Apple distribution and Flatpak requirements during specification preparation. No individual engine or package has been compiled or audited here. All dependency and feature selections require implementation-time verification.

## 17 | Common repository agent instructions

Use the following as the baseline AGENTS.md in each source tree and adapt only its repository-specific scope. Keep the full specification authoritative.

### AGENTS.md | Common project rules

Read MASTER_SPECIFICATION.md or its repository-specific canonical copy and docs/STATUS.md before changing code. The specification controls; report contradictions instead of silently choosing. Establish all four independent apps and Localwright together. Keep Localwright independently reusable by third-party applications; use the localwright repository and localwright-* Cargo package names consistently. Use Rust-first, Iced native GUI, private runtime only if needed, app-specific heavy engines. No Electron, WebView, Chromium, Tauri, web frontend, mandatory network, telemetry, ads, account, subscription or premium features. No system Python/PATH modifications, global installs, or runtime downloads. No iPad or Intel Mac work. Support five native targets, offline EXE and MSI installers plus portable ZIP for Windows x64/ARM64, Flatpak from start, AUR later, no Snap. All 15 target package combinations per studio require separate evidence. Maintain original files by default and verify real functionality.

Prefer small changes, pinned audited dependencies and exact Localwright revisions. Use standard public GitHub Actions for checks, tests, packaging and release preparation. Use local execution when necessary or cheaper. No paid services, destructive Git actions, unrelated repo access or unauthorized public releases. Stop after repeated failure and report evidence. Never fabricate build/test output, format support, DOI, authorship, signatures or screenshots. Keep statuses and docs current. No secrets in code/logs or untrusted PR execution. Protect release credentials. Report security, license, dependency and platform blockers. All public prose should be natural and avoid em dashes. Citation and donations are voluntary, never part of file output unless requested. Treat user data and imported content as untrusted; do not execute scripts/macros. See master handbook for full acceptance criteria.

GitHub is canonical. GitLab mirrors are deferred until after the first verified releases and explicit approval. Zenodo is the default archival provider; Figshare is an optional alternative. No mirrored credentials or duplicate DOIs.
