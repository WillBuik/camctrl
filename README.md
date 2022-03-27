CamCtrl
-------

Basic ONVIF camera control CLI based on the
[ONVIF-rs](https://github.com/lumeohq/onvif-rs) project.

Usage
=====

Detect cameras: `camctrl probe`

Show camera info: `camctrl --uri URI info`

### Additional Commands

The CLI also supports simple commands for basic user management
and rebooting remote cameras. See `camctrl help` for more info.

Installation
============

You can use `cargo` to install this tool if you don't want to
use it from source. Install with
`cargo install --git https://github.com/WillBuik/camctrl.git`
