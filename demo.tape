# Where should we write the GIF?
Output demo.gif

# Set up terminal.
Set FontSize 8
Set Width 1200
Set Height 600

# Show results of getting artworks.
Type "acres-cli artworks | jq"
Sleep 500ms
Enter
Sleep 500ms

# Start up TUI.
Type "clear"
Enter
Type "acres-cli artworks > ./scratch/artworks.json"
Enter
Type "acres-tui ./scratch/artworks.json"
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


