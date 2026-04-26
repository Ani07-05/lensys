package lensys

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

const DefaultBaseURL = "http://127.0.0.1:17373"

type Client struct {
	baseURL string
	http    *http.Client
}

func NewClient(baseURL string) *Client {
	if strings.TrimSpace(baseURL) == "" {
		baseURL = DefaultBaseURL
	}
	return &Client{
		baseURL: strings.TrimRight(baseURL, "/"),
		http: &http.Client{
			Timeout: 45 * time.Second,
		},
	}
}

type APIError struct {
	Error string `json:"error"`
}

type HealthResponse struct {
	OK  bool   `json:"ok"`
	API string `json:"api"`
}

type Symbol struct {
	Kind string `json:"kind"`
	Name string `json:"name"`
	Line int    `json:"line"`
}

type CodeContext struct {
	FilePath    *string  `json:"file_path"`
	FileName    *string  `json:"file_name"`
	Language    *string  `json:"language"`
	Content     *string  `json:"content"`
	WindowTitle string   `json:"window_title"`
	ActiveApp   string   `json:"active_app"`
	IsIDE       bool     `json:"is_ide"`
	Symbols     []Symbol `json:"symbols"`
}

type SearchResult struct {
	Title   string `json:"title"`
	URL     string `json:"url"`
	Content string `json:"content"`
}

type TranscriptEntry struct {
	Role      string `json:"role"`
	Text      string `json:"text"`
	Kind      string `json:"kind"`
	Timestamp string `json:"timestamp"`
}

type CodeActionProposal struct {
	Summary           string   `json:"summary"`
	Confidence        float64  `json:"confidence"`
	TargetFile        *string  `json:"target_file"`
	OldText           string   `json:"old_text"`
	Replacement       string   `json:"replacement"`
	NeedsConfirmation bool     `json:"needs_confirmation"`
	RiskNotes         []string `json:"risk_notes"`
}

type ApplyCodeActionResult struct {
	TargetFile string `json:"target_file"`
	Changed    bool   `json:"changed"`
	Message    string `json:"message"`
}

type WriteResponse struct {
	Proposal CodeActionProposal    `json:"proposal"`
	Result   ApplyCodeActionResult `json:"result"`
}

type ReadResponse struct {
	Path    string `json:"path"`
	Content string `json:"content"`
}

type RgResponse struct {
	Command string `json:"command"`
	Output  string `json:"output"`
}

func (c *Client) Health(ctx context.Context) (HealthResponse, error) {
	var out HealthResponse
	err := c.get(ctx, "/health", &out)
	return out, err
}

func (c *Client) Context(ctx context.Context) (CodeContext, error) {
	var out CodeContext
	err := c.get(ctx, "/context", &out)
	return out, err
}

func (c *Client) Capture(ctx context.Context) (CodeContext, error) {
	var out CodeContext
	err := c.post(ctx, "/capture", nil, &out)
	return out, err
}

func (c *Client) Ask(ctx context.Context, question string) (string, error) {
	var out struct {
		Answer string `json:"answer"`
	}
	err := c.post(ctx, "/ask", map[string]string{"question": question}, &out)
	return out.Answer, err
}

func (c *Client) Write(ctx context.Context, instruction string) (WriteResponse, error) {
	var out WriteResponse
	err := c.post(ctx, "/write", map[string]string{"instruction": instruction}, &out)
	return out, err
}

func (c *Client) Search(ctx context.Context, query string) ([]SearchResult, error) {
	var out []SearchResult
	err := c.post(ctx, "/web/search", map[string]string{"query": query}, &out)
	return out, err
}

func (c *Client) Transcript(ctx context.Context) ([]TranscriptEntry, error) {
	var out []TranscriptEntry
	err := c.get(ctx, "/transcript", &out)
	return out, err
}

func (c *Client) ClearTranscript(ctx context.Context) error {
	var out map[string]bool
	return c.post(ctx, "/transcript/clear", nil, &out)
}

func (c *Client) ReadFile(ctx context.Context, path string) (ReadResponse, error) {
	var out ReadResponse
	err := c.post(ctx, "/tools/read", map[string]string{"path": path}, &out)
	return out, err
}

func (c *Client) Rg(ctx context.Context, query, path string) (RgResponse, error) {
	body := map[string]string{"query": query}
	if strings.TrimSpace(path) != "" {
		body["path"] = path
	}
	var out RgResponse
	err := c.post(ctx, "/tools/rg", body, &out)
	return out, err
}

func (c *Client) get(ctx context.Context, path string, out any) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, c.baseURL+path, nil)
	if err != nil {
		return err
	}
	return c.do(req, out)
}

func (c *Client) post(ctx context.Context, path string, body any, out any) error {
	var reader io.Reader
	if body != nil {
		var buf bytes.Buffer
		if err := json.NewEncoder(&buf).Encode(body); err != nil {
			return err
		}
		reader = &buf
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.baseURL+path, reader)
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")
	return c.do(req, out)
}

func (c *Client) do(req *http.Request, out any) error {
	resp, err := c.http.Do(req)
	if err != nil {
		return fmt.Errorf("API offline at %s: %w", c.baseURL, err)
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return err
	}
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		var apiErr APIError
		if json.Unmarshal(data, &apiErr) == nil && apiErr.Error != "" {
			return errors.New(apiErr.Error)
		}
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, strings.TrimSpace(string(data)))
	}
	if out == nil || len(data) == 0 {
		return nil
	}
	return json.Unmarshal(data, out)
}
