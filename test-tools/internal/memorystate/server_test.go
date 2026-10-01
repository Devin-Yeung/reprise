package memorystate

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestStateHandlerTracksMutations(t *testing.T) {
	store, err := newStateStore()
	if err != nil {
		t.Fatal(err)
	}
	handler := newHandler(store)
	initial := store.snapshot()
	if len(initial.BootNonce) != 64 || initial.Value != nil || initial.Revision != 0 {
		t.Fatalf("unexpected initial state: %#v", initial)
	}

	mutated := serveJSON(t, handler, http.MethodPost, "/mutate", `{"value":"fresh"}`)
	if mutated.BootNonce != initial.BootNonce || mutated.Value == nil || *mutated.Value != "fresh" || mutated.Revision != 1 {
		t.Fatalf("unexpected mutation response: %#v", mutated)
	}

	read := serveJSON(t, handler, http.MethodGet, "/state", "")
	if read.BootNonce != mutated.BootNonce || read.Value == nil || *read.Value != "fresh" || read.Revision != 1 {
		t.Fatalf("unexpected read response: %#v", read)
	}
}

func TestMutateRejectsInvalidBodies(t *testing.T) {
	tests := []struct {
		name       string
		body       string
		statusCode int
	}{
		{name: "malformed", body: `{"value":`, statusCode: http.StatusBadRequest},
		{name: "missing", body: `{}`, statusCode: http.StatusBadRequest},
		{name: "null", body: `{"value":null}`, statusCode: http.StatusBadRequest},
		{name: "trailing JSON", body: `{"value":"ok"} {}`, statusCode: http.StatusBadRequest},
		{name: "oversized", body: `{"value":"` + strings.Repeat("x", maxBodyBytes) + `"}`, statusCode: http.StatusRequestEntityTooLarge},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			store, err := newStateStore()
			if err != nil {
				t.Fatal(err)
			}
			request := httptest.NewRequest(http.MethodPost, "/mutate", strings.NewReader(test.body))
			response := httptest.NewRecorder()
			newHandler(store).ServeHTTP(response, request)
			if response.Code != test.statusCode {
				t.Fatalf("status = %d, want %d; body=%q", response.Code, test.statusCode, response.Body.String())
			}
			if state := store.snapshot(); state.Value != nil || state.Revision != 0 {
				t.Fatalf("invalid request mutated state: %#v", state)
			}
		})
	}
}

func serveJSON(t *testing.T, handler http.Handler, method, path, body string) State {
	t.Helper()
	request := httptest.NewRequest(method, path, strings.NewReader(body))
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusOK {
		t.Fatalf("status = %d, want %d; body=%q", response.Code, http.StatusOK, response.Body.String())
	}
	var state State
	if err := json.NewDecoder(response.Body).Decode(&state); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	return state
}
