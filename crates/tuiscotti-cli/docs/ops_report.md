# `ops_report`

`report`: a static HTML index from saved candidate evidence.

The typed [`tuiscotti::write_report_index`] operation walks the
artifact root and writes the index; this handler only maps its
outcome to the `tuiscotti.cli.v1` result. Nothing here captures state or
changes approvals: the baseline root is never even opened.

Run `report` and return the process exit code.
