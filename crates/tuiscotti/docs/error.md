# Module

Facade error: one typed `Error`/`Result` over every fallible operation.

Each variant retains its typed source; nothing is erased to a string.
`?` converts automatically via the [`From`] impls.

D07: the removed live engine, local renderer, and adapter crates have no
variants here. Live failures surface through the upstream `tui-test`
error types in application test code, never through this facade.

# Result

Fallible-operation result over the facade [`Error`].

# Error

Every failure a facade operation can report, with sources retained.

# `View(crate::view::ViewError),`

Pure-view capture or conversion failed.

# `Sample(crate::sample::SampleError),`

Sample construction, validation, or rendering failed (D02).

# `Check(crate::check::CheckError),`

Frozen comparison or candidate evidence failed (D03).

# `Report(crate::report::ReportError),`

Static HTML report rendering failed (D03).

# `Profile(crate::render_profile::ProfileError),`

Render profile lookup or build failed (D02).

# `Approve(crate::approve::ApproveError),`

Approval transaction failed (D04).

# `Io(std::io::Error),`

Filesystem failure.
