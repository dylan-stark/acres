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

# Select fourth image and adjust alphabet
Enter
Escape
Sleep 1s
Down
Sleep 1s
Down
Sleep 1s
Down
Sleep 1s
Enter
Sleep 1s
Escape
Sleep 2s
Type "a"
Sleep 1s
Down
Sleep 1s
Enter
Sleep 1s
Escape
Sleep 1s
Type "a"
Sleep 1s
Enter
Sleep 1s
Down
Sleep 1s
Enter
Sleep 1s
Down
Sleep 1s
Enter
Sleep 1s
Escape

Sleep 3s

Type "m"
Sleep 1s
Up
Sleep 1s
Up
Sleep 1s
Enter
Sleep 1s
Escape

Sleep 3s


