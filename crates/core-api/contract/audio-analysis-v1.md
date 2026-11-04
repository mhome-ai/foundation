# Finite AudioBridge audio analysis v1

This optional internal Node capability adds no App Facade route, permission,
device selection, passive listener, or new Core protocol revision. AudioBridge
remains on Core protocol 3. Core selects the current authenticated AudioBridge
Node and authorizes the Artifact's tenant and scope before requesting analysis.

`/audiobridge/audio/analysis/capabilities` accepts `{}`. Discovery must not load
a model, open a device, or start inference. `supported` describes the interface;
`loaded` means this process has actually verified and initialized its model.
`expectedModel` preserves the expected identity even when not loaded. The
`format` and `limits` fields describe the fixed version 1 contract.

`/audiobridge/audio/analyze` accepts `{ "audio": MediaReference }`. Only an Audio
Artifact with MIME `audio/wav` is admissible. Its body must be an exact 44-byte
RIFF/WAVE header followed by 4,800–480,000 signed PCM16 little-endian mono
samples at 16 kHz. RIFF/data byte counts, format, Artifact size and SHA-256 must
match; extra chunks, truncation and recovery-by-skipping are forbidden. Optional
Artifact duration is floor(sampleCount × 1000 / 16000). Validate the metadata
size limit of 960,044 bytes before resolving or allocating the body.

`AudioAnalysisRequest::reference` / `sample_count` / `validate_wav` share these
checks. `validate_wav` returns the original PCM byte slice without decoding or
padding allocation. They do not replace authenticated Artifact resolution or
tenant/scope authorization. There are no inline samples, caller URLs or paths.

The response is a strict `status` union with `analysisVersion: 1` and the exact
input `audio` reference. `unknown` carries only one of `unavailable`, `busy`,
`timeout`, `unsupportedAudio`, `invalidAudio`, `insufficientAudio`,
`inferenceFailed`, or `incompleteScores`. It makes no coverage/score claim.
Unknown may describe an otherwise inadmissible Audio Artifact; malformed DTOs
and non-Audio references still use the ordinary request-error boundary.

`analyzed` contains `model { id, modelSha256, labelsSha256 }`, `sampleRateHz`,
`sampleCount`, the full `classes` catalog, and `windows`. Hashes are lowercase
64-digit hexadecimal strings. IDs and labels are nonblank and at most 256 UTF-8
bytes; catalog indexes are exactly 0 through N−1, ordered, with 1–1,024 entries.
Labels are retained verbatim; they are not business dispositions. The producer
verifies the raw label asset against `labelsSha256` before deriving this catalog.

Each window contains `offsetSamples`, `validSamples`, `paddingSamples`, and one
finite score in [0,1] per class, in index order. Missing top-k values must never
be filled with zero. `audio_analysis_coverage` defines 64,000-sample windows at
32,000-sample hops: stop at the first window covering the last actual sample.
Short/final windows are explicitly right-padded; padding is not captured audio.
The maximum input requires exactly 14 windows. Every request has fresh windows
and native stream state, independent of passive-listener episode buffers.

`AudioAnalysisResponse::validate` checks the version, metadata/sample count,
ordered catalog, exact window coverage, complete finite scores and the 512 KiB
serialized budget. `validate_for` also checks the input identity;
`validate_for_model` checks the caller's accepted model identity. Consumers using
raw JSON should call `from_json_slice`, which enforces the same cap before
deserialization, including whitespace. The schema enforces structure and local
bounds; UTF-8 byte budgets, canonical Artifact metadata, cross-field coverage,
score counts and exact identities also require these shared semantic checks.

No product model or score threshold is pinned here. Core must match its accepted
model/labels and current input revision, then apply an explicitly supported sound
policy. Empty ASR, absent passive events, weak scores, padding or an unknown
response are not evidence of irrelevant sound and cannot authorize Resume.

The producer admits at most one finite request, including fetch/load/inference,
with no waiting backlog and a five-second end-to-end budget. Busy returns
`unknown/busy`. Cancellation or timeout must retain ownership of a running
non-interruptible native call and its buffers until it finishes, discarding late
results. No model download, fallback model, device activation, or transparent
retry is implied by this contract. Model and hardware acceptance remain separate
from the ordinary contract tests.
