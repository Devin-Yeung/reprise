package memorystate

import (
	"encoding/json"
	"errors"
	"io"
	"net"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"
)

const maxBodyBytes = 4 << 10

// Serve runs the workload HTTP server on address until it exits.
func Serve(address string) error {
	store, err := newStateStore()
	if err != nil {
		return err
	}

	listener, err := net.Listen("tcp", address)
	if err != nil {
		return err
	}
	defer listener.Close()

	server := &http.Server{
		Handler:           newHandler(store),
		ReadHeaderTimeout: 5 * time.Second,
	}
	return server.Serve(listener)
}

func newHandler(store *stateStore) http.Handler {
	router := chi.NewRouter()
	router.Get("/state", func(w http.ResponseWriter, _ *http.Request) {
		writeJSON(w, store.snapshot())
	})
	router.Post("/mutate", func(w http.ResponseWriter, r *http.Request) {
		r.Body = http.MaxBytesReader(w, r.Body, maxBodyBytes)
		var body struct {
			Value *string `json:"value"`
		}
		decoder := json.NewDecoder(r.Body)
		if err := decoder.Decode(&body); err != nil {
			var tooLarge *http.MaxBytesError
			if errors.As(err, &tooLarge) {
				http.Error(w, "request body too large", http.StatusRequestEntityTooLarge)
				return
			}
			http.Error(w, "expected a JSON string value", http.StatusBadRequest)
			return
		}
		if body.Value == nil {
			http.Error(w, "expected a JSON string value", http.StatusBadRequest)
			return
		}
		if err := decoder.Decode(new(any)); err != io.EOF {
			http.Error(w, "expected a single JSON value", http.StatusBadRequest)
			return
		}
		writeJSON(w, store.mutate(*body.Value))
	})
	return router
}

func writeJSON(w http.ResponseWriter, value State) {
	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(value)
}
