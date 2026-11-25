# Where should we write the GIF?
Output demo.gif

# Set up terminal.
Set FontSize 8
Set Width 1200
Set Height 600

# Start up TUI on the offline fixture (`make demo` serves its images locally).
Type "acres-tui acres-tui/tests/fixtures/artworks.json"
Sleep 500ms
Enter
Sleep 3s

# Move through the results: the art and record follow the cursor.
Type "j"
Sleep 2s
Type "j"
Sleep 2s

# Open help for the results pane, then close it.
Type "?"
Sleep 3s
Escape
Sleep 1s

# Cycle to quadrant blocks, then ASCII, and try a ramp.
Type "b"
Sleep 2s
Type "b"
Sleep 2s
Type "r"
Sleep 1s
Down
Sleep 500ms
Down
Sleep 500ms
Enter
Sleep 2s
Escape
Sleep 1s
Type "b"
Sleep 1s

# Search the titles for The Great Wave.
Type "/"
Sleep 500ms
Type "wave"
Sleep 500ms
Enter
Sleep 3s

# Zen: the art alone, full screen.
Type "z"
Sleep 3s
Escape
Sleep 1s

# Slide out the timing HUD, then close it.
Type "t"
Sleep 3s
Type "t"
Sleep 2s
