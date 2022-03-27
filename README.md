CamCtrl
-------

Basic ONVIF camera control CLI based on the
[ONVIF-rs](https://github.com/lumeohq/onvif-rs) project.

Usage
=====

Detect cameras: `camctrl probe`

This will detect cameras on any networks with configued IPs on
the current machine. This can be useful if your cameras are
on a dedicated VLAN.

Show camera info: `camctrl --uri URI --creds CRED_FILE info`

`CRED_FILE` must have the following format:

`[
  {
    user: "USER_NAME",
    pass: "PASSWORD"
  }
]`

If `--creds` is not specified, no credentials will be provided
to the camera, but most operations will fail without them.

### Additional Commands

The CLI also supports simple commands for basic user management
and rebooting remote cameras. See `camctrl help` for more info.

Installation
============

You can use `cargo` to install this tool if you don't want to
use it from source. Install with
`cargo install --git https://github.com/WillBuik/camctrl.git`

Limitations
===========

This tool currently only has IPv4 support.
