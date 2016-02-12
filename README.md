# YouCompleteMeFlagsDaemon
Seamless youcompleteme flags fetcher for a file

Tired of modifying .ycm_extra_conf.py ? Yeah, I know that feeling.

With this short rust program I didn't have to touch .ycm_extra_conf.py for about a year now and I ALWAYS get auto completion automatically.

How is it so you might ask?

This short program runs a daemon that saves all flags it receives for an absolute path, therefore, whenever you open vim it asks for this daemon.

It gets flags from compile_commands.json ( I use cmake for everything. You don't? Tough, you probably won't enjoy autoflags easily :( ). This daemon itself does not monitor changes of that file, but rather, notifyhome script does that (modify it according to your environment).

But wait, what about headers? Only sources are compiled right?

Yeah, and this is the main and most important feature of this daemon. This daemon calls compiler from compile_commands.json with -M flag to receive all the headers it used for compilation and applies the same flags to the headers as it did for source.


## How to run it?

1. build rust daemon (obviously)
2. launch daemon executable (built by rust) anywhere, it will open port 7777 (not configurable ATM, would appreciate patch for this)
3. launch *EDITED FOR YOUR ENVIRONMENT* notifyhome script to keep monitoring for file changes involving compile_commands.json in the directory in where you build
4. Use ycm_extra_conf.py privided in this directory (globally, not per project)
5. Never worry about messing with .ycm_extra_conf.py again

I launch daemon and notifyhome from cron:
~~~~~~~
@reboot /home/deividas/bin/daemonycm > /home/deividas/Desktop/ramdisk/ycmdaemon.log
@reboot /home/deividas/bin/notifyhome > /home/deividas/Desktop/ramdisk/notify.log
~~~~~~~

My example configuration for using ycm_extra_conf.py:
~~~~~~~
let g:ycm_global_ycm_extra_conf = '/home/deividas/.vim/misc/ycm_extra_conf.py'
let g:ycm_confirm_extra_conf=1
~~~~~~~

How to make cmake export compile_commands.json:
~~~~~~~
SET( CMAKE_EXPORT_COMPILE_COMMANDS yes )
~~~~~~~
