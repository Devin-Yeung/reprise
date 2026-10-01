package memorystate

import (
	"crypto/rand"
	"encoding/hex"
	"sync"
)

// State is the in-memory state exposed by the workload's HTTP API and CLI.
type State struct {
	BootNonce string  `json:"boot_nonce"`
	Value     *string `json:"value"`
	Revision  uint64  `json:"revision"`
}

type stateStore struct {
	mu    sync.Mutex
	state State
}

func newStateStore() (*stateStore, error) {
	nonce := make([]byte, 32)
	if _, err := rand.Read(nonce); err != nil {
		return nil, err
	}
	return &stateStore{state: State{BootNonce: hex.EncodeToString(nonce)}}, nil
}

func (s *stateStore) snapshot() State {
	s.mu.Lock()
	defer s.mu.Unlock()
	return cloneState(s.state)
}

func (s *stateStore) mutate(value string) State {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.state.Value = &value
	s.state.Revision++
	return cloneState(s.state)
}

func cloneState(state State) State {
	if state.Value != nil {
		value := *state.Value
		state.Value = &value
	}
	return state
}
