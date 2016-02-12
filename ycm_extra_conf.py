
import os
import ycm_core

def netcat(hostname, port, content):
    import socket
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect((hostname, port))
    s.sendall(content)
    s.shutdown(socket.SHUT_WR)
    outdata = ""
    while 1:
        data = s.recv(1024)
        if data == "":
            break
        outdata += data
        print "Received:", repr(data)
    print "Connection closed."
    s.close()
    return outdata

def GetFromDaemon( context, filename ):
    import shlex
    toSend = "q|" + context + "|" + filename
    out = netcat("localhost",7777,toSend)
    splitted = out.split("|")
    stripped = splitted[3].strip()
    shlexed = shlex.split(stripped)
    shlexed.pop(0)
    survivors = []
    arrLen = len(shlexed)
    wasOut = False
    for i in range(0,arrLen):
        if shlexed[i] != '-o' and not wasOut:
            survivors.append(shlexed[i])

        if shlexed[i] == '-o':
            wasOut = True
        else:
            wasOut = False

    return survivors


def FlagsForFile( filename, **kwargs ):
  fromDaemon = GetFromDaemon( "default", filename )

  return {
      'flags': fromDaemon,
      'do_cache': False
  }
