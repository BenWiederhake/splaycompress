#!/usr/bin/env python3

import collections
import subprocess
import sys

BENCHMARK_FILE = "src/splay.rs"

ALGO_SPLAYCOMPRESS = True

try:
    import brotli
    ALGO_BROTLI = True
except:
    ALGO_BROTLI = False

try:
    import bz2
    ALGO_BZ2 = True
except:
    ALGO_BZ2 = False

try:
    import ficticious_module_name_that_hopefully_does_not_exist
    ALGO_FICTICIOUS = True
except:
    ALGO_FICTICIOUS = False

try:
    import gzip
    ALGO_GZIP = True
except:
    ALGO_GZIP = False

try:
    import lz4.frame
    ALGO_LZ4 = True
except:
    ALGO_LZ4 = False

try:
    import lzma
    ALGO_LZMA = True
except:
    ALGO_LZMA = False

try:
    import zlib
    ALGO_ZLIB = True
except:
    ALGO_ZLIB = False

try:
    import zstd
    ALGO_ZSTD = True
except:
    ALGO_ZSTD = False


Algorithm = collections.namedtuple("Algorithm", ["name", "compress", "decompress"])


def splaycompress_prepare():
    subprocess.run(["cargo", "build", "--release", "-q"], check=True)


def splaycompress(args, data):
    result = subprocess.run(
        ["./target/release/splaycompress", *args],
        capture_output=True,
        input=data,
        check=True,
    )
    return result.stdout


def maybe_add(flag, name, compress, decompress, algos_list):
    if flag:
        algos_list.append(Algorithm(name, compress, decompress))
    else:
        print(f"NOTE: Compression '{name}' could not be loaded.", file=sys.stderr)


def determine_algos():
    algos = []
    maybe_add(True, "none", lambda d: d, lambda d: d, algos)
    maybe_add(ALGO_BROTLI, "brotli", lambda d: brotli.compress(d), lambda d: brotli.decompress(d), algos)
    maybe_add(ALGO_BZ2, "bz2", lambda d: bz2.compress(d), lambda d: bz2.decompress(d), algos)
    maybe_add(ALGO_FICTICIOUS, "ficticious", None, None, algos)
    maybe_add(ALGO_GZIP, "gzip", lambda d: gzip.compress(d), lambda d: gzip.decompress(d), algos)
    maybe_add(ALGO_LZ4, "lz4", lambda d: lz4.frame.compress(d), lambda d: lz4.frame.decompress(d), algos)
    maybe_add(ALGO_LZMA, "lzma", lambda d: lzma.compress(d, format=lzma.FORMAT_ALONE), lambda d: lzma.decompress(d, format=lzma.FORMAT_ALONE), algos)
    maybe_add(ALGO_LZMA, "xz", lambda d: lzma.compress(d), lambda d: lzma.decompress(d), algos)
    splaycompress_prepare()
    maybe_add(ALGO_SPLAYCOMPRESS, "jan", lambda d: splaycompress([], d), lambda d: splaycompress(["-d"], d), algos)
    maybe_add(ALGO_ZLIB, "zlib", lambda d: zlib.compress(d), lambda d: zlib.decompress(d), algos)
    maybe_add(ALGO_ZSTD, "zstd", lambda d: zstd.compress(d), lambda d: zstd.decompress(d), algos)
    return algos


def sanity_check_with(test_vector, algos):
    for algo in algos:
        compressed = algo.compress(test_vector)
        decompressed = algo.decompress(compressed)
        assert decompressed == test_vector
        recompressed = algo.compress(decompressed)
        assert recompressed == compressed


def sanity_check(algos):
    assert len(algos) >= 3, "extremely few successfully loaded?!"
    assert len(set(a.name for a in algos)) == len(algos), "duplicate name?!"
    assert len(set(a.compress for a in algos)) == len(algos), "duplicate compression function?!"
    assert len(set(a.decompress for a in algos)) == len(algos), "duplicate decompression function?!"
    for test_vector in [b"x", b"short\x00example\xff"]:
        sanity_check_with(test_vector, algos)
    print("Passed sanity check: " + ", ".join(a.name for a in algos), file=sys.stderr)


def run():
    algos = determine_algos()
    sanity_check(algos)
    with open(BENCHMARK_FILE, "rb") as fp:
        data = fp.read()
    print(",".join(a.name for a in algos))
    for i in range(len(data)):
        partial_data = data[:i]
        print(",".join(str(len(a.compress(partial_data))) for a in algos))


if __name__ == "__main__":
    run()
