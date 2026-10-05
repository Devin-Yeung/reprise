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
		Short:         "Run the in-memory test workload",
		Version:       "0.1.0",
		SilenceErrors: true,
		SilenceUsage:  true,
	}
	var socket string
	serve := &cobra.Command{
		Use:   "serve",
		Short: "Keep the workload state alive over a Unix socket",
		Args:  cobra.NoArgs,
		RunE: func(_ *cobra.Command, _ []string) error {
			listener, err := memorystate.Listen(socket)
			if err != nil {
				return err
			}
			defer listener.Close()
			return memorystate.Serve(listener)
		},
	}
	serve.Flags().StringVar(&socket, "socket", "/run/memory-state.sock", "Unix socket to listen on")
	root.AddCommand(serve)

	root.AddCommand(socketCommand("get", "Read the running workload state", cobra.NoArgs, func(socket string, _ []string) (memorystate.State, error) {
		return memorystate.Get(socket)
	}))
	root.AddCommand(socketCommand("mutate VALUE", "Replace the value and advance its revision", cobra.ExactArgs(1), func(socket string, args []string) (memorystate.State, error) {
		return memorystate.Mutate(socket, args[0])
	}))

	version := &cobra.Command{
		Use:   "version",
		Short: "Print the memory-state version",
		Args:  cobra.NoArgs,
		Run: func(cmd *cobra.Command, _ []string) {
			fmt.Fprintln(cmd.OutOrStdout(), "0.1.0")
		},
	}
	root.AddCommand(version)

	return root
}

func socketCommand(use, short string, args cobra.PositionalArgs, invoke func(string, []string) (memorystate.State, error)) *cobra.Command {
	var socket string
	command := &cobra.Command{
		Use:   use,
		Short: short,
		Args:  args,
		RunE: func(cmd *cobra.Command, args []string) error {
			state, err := invoke(socket, args)
			if err != nil {
				return err
			}
			return json.NewEncoder(cmd.OutOrStdout()).Encode(state)
		},
	}
	command.Flags().StringVar(&socket, "socket", "/run/memory-state.sock", "Unix socket of the running workload")
	return command
}
