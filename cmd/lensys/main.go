package main

import (
	"fmt"
	"os"

	tea "github.com/charmbracelet/bubbletea"

	"hackblr/internal/lensys"
)

func main() {
	program := tea.NewProgram(lensys.NewModel(), tea.WithAltScreen())
	if _, err := program.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "lensys failed: %v\n", err)
		os.Exit(1)
	}
}
