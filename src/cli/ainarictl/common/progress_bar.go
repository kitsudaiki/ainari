/**
 * @author      Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright   Apache License Version 2.0
 *
 *      Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 *      Licensed under the Apache License, Version 2.0 (the "License");
 *      you may not use this file except in compliance with the License.
 *      You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 *      Unless required by applicable law or agreed to in writing, software
 *      distributed under the License is distributed on an "AS IS" BASIS,
 *      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *      See the License for the specific language governing permissions and
 *      limitations under the License.
 */

package ainarictl_common

import (
	"fmt"
	"os"
	"strings"

	"golang.org/x/term"
)

// progressBarWidth is the number of characters of the bar itself
const progressBarWidth = 40

// ProgressBar prints the progress of an upload as bar into a single line of the terminal. It is
// written to stderr, so the result on stdout can still be parsed, also with the json-output.
// Without a terminal, for example if stderr is redirected into a file, nothing is printed.
type ProgressBar struct {
	label       string
	lastPercent int
	enabled     bool
	finished    bool
}

// NewProgressBar creates a new progress-bar, which shows the label in front of the bar.
func NewProgressBar(label string) *ProgressBar {
	return &ProgressBar{
		label:       label,
		lastPercent: -1,
		enabled:     term.IsTerminal(int(os.Stderr.Fd())),
	}
}

// Update redraws the bar with the new progress. The line is only redrawn, when the percentage
// changed, so a big upload doesn't flood the terminal. At 100% the line of the bar is finished.
func (b *ProgressBar) Update(done, total int64) {
	if !b.enabled || b.finished || total <= 0 {
		return
	}

	percent := int(done * 100 / total)
	if percent == b.lastPercent {
		return
	}
	b.lastPercent = percent

	filled := progressBarWidth * percent / 100
	bar := strings.Repeat("=", filled) + strings.Repeat(" ", progressBarWidth-filled)
	fmt.Fprintf(os.Stderr, "\r%s [%s] %3d%% %10s / %s",
		b.label, bar, percent, formatBytes(done), formatBytes(total))

	if done >= total {
		b.Finish()
	}
}

// Finish ends the line of the bar, so following output starts in a new line. It can be called
// multiple times, for example after an aborted upload, and does nothing, if the bar was never
// printed.
func (b *ProgressBar) Finish() {
	if !b.enabled || b.finished || b.lastPercent < 0 {
		return
	}
	b.finished = true
	fmt.Fprintln(os.Stderr)
}

// formatBytes converts a number of bytes into the largest fitting binary unit with one decimal.
func formatBytes(bytes int64) string {
	units := []string{"B", "KiB", "MiB", "GiB", "TiB"}
	size := float64(bytes)
	unit := 0
	for size >= 1024 && unit < len(units)-1 {
		size /= 1024
		unit++
	}
	if unit == 0 {
		return fmt.Sprintf("%d B", bytes)
	}
	return fmt.Sprintf("%.1f %s", size, units[unit])
}
