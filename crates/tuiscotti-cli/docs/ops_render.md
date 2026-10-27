# `ops_render`

`render`: `.png` or `.html` output from a saved sample, fully offline.

The typed [`tuiscotti::render_sample`] operation does the work; this
handler only reads the sample file with a bounded read and maps the
outcome to the Section 16.3 result. The `--out` extension is validated
by a clap parser before this runs (exit 2 with no JSON result), so
every failure below is operational and exits 3.

Run `render` and return the process exit code.
