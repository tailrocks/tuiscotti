# Module

Offline rendering of a saved sample (GOAL Section 16).

[`render_sample`] extracts the stored PNG from an image sample, or
re-renders a state-only sample through the built-in profile, or writes
a static `.html` page embedding the image — selected by the `--out`
extension. Nothing here starts an application: the input is saved
bytes, and the output is one file write. The CLI boundary owns
argument parsing and process exit codes; every failure here keeps
its typed shape and Section 16.3 code through [`RenderError::code`].

# `RenderError`

Every failure an offline sample render can report.

# `UnsupportedExtension` {

The output path carries neither a `.png` nor an `.html` extension.

# path: `PathBuf`,

The rejected output path, for the diagnostic.

# Sample(SampleError),

The sample bytes fail [`Sample`] validation.

# `StoredImage` {

A stored image sample carries PNG bytes that do not validate.

# detail: String,

What the stored PNG broke.

# Profile(ProfileError),

A state-only sample names a render profile that cannot be used.

# Render {

The upstream renderer refuses a state-only sample snapshot.

# source: `UpstreamRenderError`,

The upstream rejection, with its source chain.

# Io {

A filesystem failure outside sample validation.

# source: `std::io::Error`,

The filesystem failure.

# code

The stable Section 16.3 machine code for this failure.

# `RenderOutcome`

One completed offline render: the reviewed bytes written to one file.

# `test_id`

The rendered test ID.

# `candidate_sha256`

SHA-256 over the rendered sample bytes.

# reason

The render profile for a rendered state-only sample, else `None`.

# `output_path`

The written output file.

# `render_sample`

Render saved sample bytes to `out` and return the outcome.

A `.png` output holds the validated stored image, or the state-only
re-render; an `.html` output embeds it in a static page. Any other
extension is rejected before the bytes are read.

# Errors

Returns [`RenderError`] for unsupported extensions, invalid samples,
stored-image failures, unknown profiles, renderer refusals, and I/O
failures.

# `wants_png`

True for the `.png` output extension.

# `wants_html`

True for the `.html` output extension.

# `write_output`

Write the rendered bytes: raw PNG for `.png`, a static embedding
page for `.html`.

# `render_page`

One static HTML page for a rendered sample with its PNG embedded as
a data URI: no external files, no absolute paths, deterministic
output for one input.

# `decode_stored_png`

Decode and validate a stored sample PNG: the Base64 decodes, the
image parses, and the decoded dimensions match the recorded ones.

# `render_state_only`

Render a state-only sample through the built-in profile.

# `write_error`

Name the output path in a write failure, keeping the error kind.

# `image_dimensions`

Decoded PNG dimensions for the HTML page; zero when undecodable
(the PNG path already validated, so this never fails there).
