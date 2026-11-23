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
Sleep 500ms

# Open the artworks list and show the first artwork.
Escape
Sleep 1s
Enter
Sleep 1s
Escape
Sleep 3s

# Try a few ramps: letters, then minimal, then back to alphabet.
Type "a"
Sleep 1s
Down
Sleep 500ms
Down
Sleep 500ms
Enter
Sleep 2s
Down
Sleep 500ms
Down
Sleep 500ms
Enter
Sleep 2s
Up
Sleep 500ms
Up
Sleep 500ms
Up
Sleep 500ms
Up
Sleep 500ms
Enter
Sleep 1s
Escape
Sleep 2s

# Switch to The Great Wave.
Escape
Sleep 1s
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Enter
Sleep 1s
Escape
Sleep 3s

# Slide out the timing HUD, then close it.
Type "t"
Sleep 4s
Type "t"
Sleep 2s
