#!/usr/bin/env nu

const FUZZ_TARGETS = ["sum_digits"]
const FUZZ_DURATION_S_DEFAULT = 30
const AFL_NOTES_FILE = "README.txt"

def main [--duration-s: int = $FUZZ_DURATION_S_DEFAULT] {
    ^cargo afl build

    mut crash_count = 0
    for target in $FUZZ_TARGETS {
        let binary = $"target/debug/($target)"
        let out_dir = $"($target)/out"
        let crash_dir = $"($out_dir)/default/crashes"
        let regressions_dir = $"($target)/regressions"

        rm -rf $out_dir
        mkdir $out_dir
        ^cargo afl fuzz -i $"($target)/in" -o $out_dir -V $duration_s $binary

        if not ($crash_dir | path exists) {
            continue
        }

        let crashes = ls $crash_dir | where type == file and ($it.name | path basename) != $AFL_NOTES_FILE
        for crash in $crashes {
            mkdir $regressions_dir
            let minified = $regressions_dir | path join ($crash.name | path basename)
            ^cargo afl tmin -i $crash.name -o $minified $binary

            let replay = do { open --raw $minified | ^$binary } | complete
            print $"crash in ($target): ($minified)" $replay.stderr
            $crash_count += 1
        }
    }

    if $crash_count > 0 {
        error make {msg: $"($crash_count) fuzz crashes, minimized into */regressions/"}
    }
}
