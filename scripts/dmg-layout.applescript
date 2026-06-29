on run argv
	set volumeName to item 1 of argv
	set appIconX to item 2 of argv as integer
	set iconY to item 3 of argv as integer
	set appsIconX to item 4 of argv as integer
	set winWidth to item 5 of argv as integer
	set winHeight to item 6 of argv as integer
	set iconSize to item 7 of argv as integer

	set winLeft to 200
	set winTop to 120

	tell application "Finder"
		tell disk volumeName
			open
			delay 1
			set theWindow to container window
			set current view of theWindow to icon view
			try
				set toolbar visible of theWindow to false
			end try
			try
				set statusbar visible of theWindow to false
			end try
			try
				set pathbar visible of theWindow to false
			end try
			try
				set sidebar width of theWindow to 0
			end try
			set viewOptions to the icon view options of theWindow
			set arrangement of viewOptions to not arranged
			set icon size of viewOptions to iconSize
			try
				set background picture of viewOptions to file ".background:background.png"
			end try
			try
				set extension hidden of viewOptions to true
			end try
			try
				set bounds of theWindow to {winLeft, winTop, winLeft + winWidth, winTop + winHeight}
				set picture ratio of viewOptions to false
			end try
			set position of item "SpeakType.app" of theWindow to {appIconX, iconY}
			set position of item "Applications" of theWindow to {appsIconX, iconY}
			update without registering applications
			delay 2
			close
		end tell
	end tell
end run
