package lensys

import (
	"context"
	"fmt"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/textinput"
	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

type tab int

const (
	tabHome tab = iota
	tabAsk
	tabAct
	tabSearch
	tabContext
	tabTranscript
	tabTools
)

var tabs = []string{"Home", "Ask", "Act", "Search", "Context", "Transcript", "Tools"}

type Model struct {
	client *Client

	width  int
	height int
	active tab
	input  textinput.Model
	view   viewport.Model
	spin   spinner.Model

	busy       bool
	status     string
	lastError  string
	output     string
	context    CodeContext
	transcript []TranscriptEntry
}

type resultMsg struct {
	kind string
	text string
	ctx  CodeContext
	log  []TranscriptEntry
	err  error
}

type tickMsg time.Time

func NewModel() Model {
	input := textinput.New()
	input.Placeholder = "Type a prompt, path, or command..."
	input.CharLimit = 800
	input.Width = 80
	input.Prompt = "lensys> "
	input.Focus()

	view := viewport.New(80, 18)
	view.Style = lipgloss.NewStyle().Padding(1, 2)

	spin := spinner.New()
	spin.Spinner = spinner.Dot

	return Model{
		client: NewClient(DefaultBaseURL),
		input:  input,
		view:   view,
		spin:   spin,
		status: "checking API...",
		output: "Welcome to lensys. Start the Tauri app to bring the local AI API online.",
	}
}

func (m Model) Init() tea.Cmd {
	return tea.Batch(m.spin.Tick, m.healthCmd(), m.contextCmd(false), m.transcriptCmd())
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd

	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.input.Width = max(20, msg.Width-16)
		m.view.Width = max(20, msg.Width-4)
		m.view.Height = max(8, msg.Height-10)
		m.refreshView()

	case tea.KeyMsg:
		switch msg.String() {
		case "ctrl+c", "q":
			return m, tea.Quit
		case "left", "h":
			m.active = (m.active + tab(len(tabs)) - 1) % tab(len(tabs))
			m.prepareInput()
			m.refreshView()
		case "right", "l", "tab":
			m.active = (m.active + 1) % tab(len(tabs))
			m.prepareInput()
			m.refreshView()
		case "r":
			m.busy = true
			cmds = append(cmds, m.healthCmd(), m.contextCmd(false))
			if m.active == tabTranscript || m.active == tabHome {
				cmds = append(cmds, m.transcriptCmd())
			}
		case "c":
			if m.active == tabContext {
				m.busy = true
				cmds = append(cmds, m.contextCmd(true))
			} else if m.active == tabTranscript {
				m.busy = true
				cmds = append(cmds, m.clearTranscriptCmd())
			}
		case "enter":
			cmd := m.submit()
			if cmd != nil {
				m.busy = true
				cmds = append(cmds, cmd)
			}
		}

	case spinner.TickMsg:
		var cmd tea.Cmd
		m.spin, cmd = m.spin.Update(msg)
		cmds = append(cmds, cmd)

	case resultMsg:
		m.busy = false
		if msg.err != nil {
			m.lastError = msg.err.Error()
			if msg.kind == "health" {
				m.status = "API offline"
				m.output = msg.err.Error()
			} else {
				m.output = fmt.Sprintf("%s request failed: %s", msg.kind, msg.err.Error())
			}
		} else {
			m.lastError = ""
			if msg.kind == "health" {
				m.status = msg.text
			}
			if msg.kind == "context" || msg.kind == "capture" {
				m.context = msg.ctx
			}
			if msg.kind == "transcript" {
				m.transcript = msg.log
			}
			if msg.text != "" {
				m.output = msg.text
			}
		}
		m.refreshView()

	case tickMsg:
		cmds = append(cmds, m.healthCmd())
	}

	var cmd tea.Cmd
	m.input, cmd = m.input.Update(msg)
	cmds = append(cmds, cmd)
	m.view, cmd = m.view.Update(msg)
	cmds = append(cmds, cmd)

	return m, tea.Batch(cmds...)
}

func (m Model) View() string {
	if m.width == 0 {
		return "lensys loading..."
	}

	header := titleStyle.Render("LENSYS") + "  " + statusStyle.Render(m.status)
	if m.busy {
		header += "  " + busyStyle.Render(m.spin.View()+" working")
	}

	tabBar := make([]string, len(tabs))
	for i, name := range tabs {
		style := tabStyle
		if tab(i) == m.active {
			style = activeTabStyle
		}
		tabBar[i] = style.Render(name)
	}

	help := subtleStyle.Render("tab/right/left: switch  r: refresh  enter: run  c: capture/clear  q: quit")
	if m.active == tabHome || m.active == tabContext || m.active == tabTranscript {
		return lipgloss.JoinVertical(lipgloss.Left,
			header,
			lipgloss.JoinHorizontal(lipgloss.Top, tabBar...),
			m.view.View(),
			help,
		)
	}

	return lipgloss.JoinVertical(lipgloss.Left,
		header,
		lipgloss.JoinHorizontal(lipgloss.Top, tabBar...),
		m.view.View(),
		m.input.View(),
		help,
	)
}

func (m *Model) prepareInput() {
	m.input.SetValue("")
	switch m.active {
	case tabAsk:
		m.input.Placeholder = "Ask the AI about the active context..."
	case tabAct:
		m.input.Placeholder = "Describe the code change; leave blank is supported by the API but enter sends non-empty text."
	case tabSearch:
		m.input.Placeholder = "Search docs, errors, APIs..."
	case tabTools:
		m.input.Placeholder = "read path/to/file  OR  rg query [path]"
	default:
		m.input.Placeholder = "Type a prompt, path, or command..."
	}
}

func (m *Model) refreshView() {
	switch m.active {
	case tabHome:
		m.view.SetContent(m.homeView())
	case tabContext:
		m.view.SetContent(m.contextView())
	case tabTranscript:
		m.view.SetContent(m.transcriptView())
	default:
		m.view.SetContent(m.output)
	}
}

func (m Model) homeView() string {
	ascii := `
 _      _____ _   _ _______     _______
| |    | ____| \ | / ___\ \   / / ____|
| |    |  _| |  \| \___ \\ \ / /|  _|
| |___ | |___| |\  |___) |\ V / | |___
|_____||_____|_| \_|____/  \_/  |_____|
`
	ctx := contextSummary(m.context)
	recent := "No transcript yet."
	if len(m.transcript) > 0 {
		last := m.transcript[len(m.transcript)-1]
		recent = fmt.Sprintf("%s: %s", last.Role, clamp(last.Text, 240))
	}
	return fmt.Sprintf("%s\n%s\n\nContext: %s\nRecent: %s\n\nUse Ask, Act, and Search for AI calls. Context captures the active editor selection. Tools can read files or run ripgrep through the local API.",
		asciiStyle.Render(ascii),
		statusStyle.Render(m.status),
		ctx,
		recent,
	)
}

func (m Model) contextView() string {
	return fmt.Sprintf("Context\n\n%s\n\nSymbols: %s\n\nPress r to refresh active context or c to capture the current selection through the clipboard.",
		contextSummary(m.context),
		symbolSummary(m.context.Symbols),
	)
}

func (m Model) transcriptView() string {
	if len(m.transcript) == 0 {
		return "Transcript is empty.\n\nPress r to refresh or c to clear."
	}
	var b strings.Builder
	b.WriteString("Transcript\n\n")
	for _, entry := range m.transcript {
		fmt.Fprintf(&b, "[%s] %s: %s\n\n", entry.Kind, entry.Role, entry.Text)
	}
	b.WriteString("Press r to refresh or c to clear.")
	return b.String()
}

func (m Model) submit() tea.Cmd {
	value := strings.TrimSpace(m.input.Value())
	switch m.active {
	case tabAsk:
		if value == "" {
			return nil
		}
		m.input.SetValue("")
		return m.askCmd(value)
	case tabAct:
		m.input.SetValue("")
		return m.writeCmd(value)
	case tabSearch:
		if value == "" {
			return nil
		}
		m.input.SetValue("")
		return m.searchCmd(value)
	case tabTools:
		if value == "" {
			return nil
		}
		m.input.SetValue("")
		return m.toolsCmd(value)
	default:
		return nil
	}
}

func (m Model) healthCmd() tea.Cmd {
	return func() tea.Msg {
		ctx, cancel := context.WithTimeout(context.Background(), 4*time.Second)
		defer cancel()
		health, err := m.client.Health(ctx)
		if err != nil {
			return resultMsg{kind: "health", err: err}
		}
		return resultMsg{kind: "health", text: fmt.Sprintf("%s API online", health.API)}
	}
}

func (m Model) contextCmd(capture bool) tea.Cmd {
	return func() tea.Msg {
		ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
		defer cancel()
		var (
			code CodeContext
			err  error
		)
		if capture {
			code, err = m.client.Capture(ctx)
		} else {
			code, err = m.client.Context(ctx)
		}
		kind := "context"
		if capture {
			kind = "capture"
		}
		if err != nil {
			return resultMsg{kind: kind, err: err}
		}
		return resultMsg{kind: kind, ctx: code, text: fmt.Sprintf("Captured %s", contextSummary(code))}
	}
}

func (m Model) askCmd(question string) tea.Cmd {
	return func() tea.Msg {
		ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
		defer cancel()
		answer, err := m.client.Ask(ctx, question)
		return resultMsg{kind: "ask", text: answer, err: err}
	}
}

func (m Model) writeCmd(instruction string) tea.Cmd {
	return func() tea.Msg {
		ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
		defer cancel()
		result, err := m.client.Write(ctx, instruction)
		if err != nil {
			return resultMsg{kind: "act", err: err}
		}
		target := "(no target)"
		if result.Proposal.TargetFile != nil {
			target = *result.Proposal.TargetFile
		}
		text := fmt.Sprintf("Proposal: %s\nTarget: %s\nConfidence: %.2f\nChanged: %t\nResult: %s",
			result.Proposal.Summary,
			target,
			result.Proposal.Confidence,
			result.Result.Changed,
			result.Result.Message,
		)
		return resultMsg{kind: "act", text: text}
	}
}

func (m Model) searchCmd(query string) tea.Cmd {
	return func() tea.Msg {
		ctx, cancel := context.WithTimeout(context.Background(), 45*time.Second)
		defer cancel()
		results, err := m.client.Search(ctx, query)
		if err != nil {
			return resultMsg{kind: "search", err: err}
		}
		if len(results) == 0 {
			return resultMsg{kind: "search", text: "No search results."}
		}
		var b strings.Builder
		for i, result := range results {
			fmt.Fprintf(&b, "%d. %s\n%s\n%s\n\n", i+1, result.Title, result.URL, result.Content)
		}
		return resultMsg{kind: "search", text: b.String()}
	}
}

func (m Model) transcriptCmd() tea.Cmd {
	return func() tea.Msg {
		ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
		defer cancel()
		log, err := m.client.Transcript(ctx)
		return resultMsg{kind: "transcript", log: log, err: err}
	}
}

func (m Model) clearTranscriptCmd() tea.Cmd {
	return func() tea.Msg {
		ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
		defer cancel()
		err := m.client.ClearTranscript(ctx)
		return resultMsg{kind: "transcript", log: nil, text: "Transcript cleared.", err: err}
	}
}

func (m Model) toolsCmd(command string) tea.Cmd {
	return func() tea.Msg {
		ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
		defer cancel()
		fields := strings.Fields(command)
		if len(fields) < 2 {
			return resultMsg{kind: "tools", err: fmt.Errorf("use: read path/to/file  OR  rg query [path]")}
		}
		switch fields[0] {
		case "read":
			result, err := m.client.ReadFile(ctx, strings.Join(fields[1:], " "))
			if err != nil {
				return resultMsg{kind: "tools", err: err}
			}
			return resultMsg{kind: "tools", text: fmt.Sprintf("%s\n\n%s", result.Path, result.Content)}
		case "rg":
			query := fields[1]
			path := ""
			if len(fields) > 2 {
				path = strings.Join(fields[2:], " ")
			}
			result, err := m.client.Rg(ctx, query, path)
			if err != nil {
				return resultMsg{kind: "tools", err: err}
			}
			return resultMsg{kind: "tools", text: fmt.Sprintf("$ %s\n\n%s", result.Command, result.Output)}
		default:
			return resultMsg{kind: "tools", err: fmt.Errorf("unknown tool %q", fields[0])}
		}
	}
}

func contextSummary(ctx CodeContext) string {
	target := valueOr(ctx.FileName, valueOr(ctx.FilePath, ctx.WindowTitle))
	if strings.TrimSpace(target) == "" {
		target = "No active context"
	}
	lang := valueOr(ctx.Language, "")
	if lang != "" {
		target += " (" + lang + ")"
	}
	if ctx.Content != nil && *ctx.Content != "" {
		lines := strings.Count(*ctx.Content, "\n") + 1
		target += fmt.Sprintf(" - %d lines", lines)
	}
	if ctx.ActiveApp != "" {
		target += " via " + ctx.ActiveApp
	}
	return target
}

func symbolSummary(symbols []Symbol) string {
	if len(symbols) == 0 {
		return "none"
	}
	parts := make([]string, 0, min(len(symbols), 8))
	for _, symbol := range symbols[:min(len(symbols), 8)] {
		parts = append(parts, fmt.Sprintf("%s %s", symbol.Kind, symbol.Name))
	}
	return strings.Join(parts, ", ")
}

func valueOr(value *string, fallback string) string {
	if value == nil || *value == "" {
		return fallback
	}
	return *value
}

func clamp(text string, limit int) string {
	text = strings.TrimSpace(text)
	if len(text) <= limit {
		return text
	}
	return text[:limit-1] + "..."
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}

var (
	titleStyle     = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("86")).Margin(1, 2, 0, 2)
	statusStyle    = lipgloss.NewStyle().Foreground(lipgloss.Color("151"))
	busyStyle      = lipgloss.NewStyle().Foreground(lipgloss.Color("229"))
	subtleStyle    = lipgloss.NewStyle().Foreground(lipgloss.Color("244")).MarginLeft(2)
	asciiStyle     = lipgloss.NewStyle().Foreground(lipgloss.Color("87")).Bold(true)
	tabStyle       = lipgloss.NewStyle().Foreground(lipgloss.Color("246")).Padding(0, 2).MarginLeft(1)
	activeTabStyle = lipgloss.NewStyle().Foreground(lipgloss.Color("230")).Background(lipgloss.Color("62")).Padding(0, 2).MarginLeft(1)
)
