#!/usr/bin/python3

import dbus

def main():
	session = dbus.SessionBus()
	proxy = session.get_object('space.unmade.GlobalMenu', '/space/unmade/GlobalMenu')
	proxy.EmitHudActivated()

if __name__ == "__main__":
	main()
