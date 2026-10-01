// memory-state is a test workload; its nonce and mutations live only in RAM.
package main

import (
	"encoding/json"
	"fmt"
	"os"

	"reprise/test-tools/internal/memorystate"

	"github.com/spf13/cobra"
)

func main() {
	if err := newRootCommand().Execute(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func newRootCommand() *cobra.Command {
	root := &cobra.Command{
		Use:           "memory-state",
		Short:         "Run and inspect the in-memory test workload",
		SilenceErrors: true,
		SilenceUsage:  true,
	}
	root.AddCommand(
		&cobra.Command{
			Use:   "start",
			Short: "Start the workload server in the background",
			Args:  cobra.NoArgs,
			RunE: func(_ *cobra.Command, _ []string) error {
				state, err := memorystate.Start()
				if err != nil {
					return err
				}
				return writeState(state)
			},
		},
		&cobra.Command{
			Use:   "serve",
			Short: "Run the workload server in the foreground",
			Args:  cobra.NoArgs,
			RunE: func(_ *cobra.Command, _ []string) error {
				return memorystate.Serve()
			},
		},
		&cobra.Command{
			Use:   "state",
			Short: "Read the current workload state",
			Args:  cobra.NoArgs,
			RunE: func(_ *cobra.Command, _ []string) error {
				state, err := memorystate.GetState()
				if err != nil {
					return err
				}
				return writeState(state)
			},
		},
		&cobra.Command{
			Use:   "mutate VALUE",
			Short: "Set the workload value",
			Args:  cobra.ExactArgs(1),
			RunE: func(_ *cobra.Command, args []string) error {
				state, err := memorystate.Mutate(args[0])
				if err != nil {
					return err
				}
				return writeState(state)
			},
		},
	)
	return root
}

func writeState(state memorystate.State) error {
	return json.NewEncoder(os.Stdout).Encode(state)
}
