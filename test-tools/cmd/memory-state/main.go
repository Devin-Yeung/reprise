// memory-state is a test workload; its nonce and mutations live only in RAM.
package main

import (
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
	var listen string
	serve := &cobra.Command{
		Use:   "serve",
		Short: "Serve the workload state over HTTP in the foreground",
		Args:  cobra.NoArgs,
		RunE: func(_ *cobra.Command, _ []string) error {
			return memorystate.Serve(listen)
		},
	}
	serve.Flags().StringVar(&listen, "listen", "127.0.0.1:8765", "TCP address to listen on")
	root.AddCommand(serve)

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
