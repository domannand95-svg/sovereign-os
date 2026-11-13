#!/usr/bin/env python3
"""Offline source-pinned characterization; never a released-reader certificate."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile

BASELINE = 'f8a10a3c639e4b8911f6b7069e1565d5eb99c48f'
TREE = 'b1b367e90b1565d699b4ea92c226930efa1495b7'
LOCK = 'e0fc3ed947a8d348e2d472db9e5ec22d834cb839070ddc71087701afea20e567'
FIXTURE = '23aeb15569569d96c1aa57a037964aff218814b79dd5ce3fdde5fa8e362c3fdd'
FORMAT = 'sovereign.service-evidence.v2'
PROFILE = 'intent-nfkc-u17-v1'
DOMAIN = 'SOVEREIGN_OS:SERVICE_EVIDENCE:PROFILED:v2'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def checked(args, **kwargs):
    result = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            check=False, timeout=1200, **kwargs)
    if result.returncode:
        raise RuntimeError(f'{args[0]} failed ({result.returncode}): '
                           + result.stderr.decode(errors='replace')[-12000:])
    return result.stdout


def require_hash(data, expected, label):
    if digest(data) != expected:
        raise RuntimeError(f'{label} SHA-256 mismatch')


def envelope(payload):
    values = [DOMAIN, FORMAT, PROFILE, payload.decode('utf-8')]
    framed = b''.join(str(len(v.encode())).encode() + b':' + v.encode() for v in values)
    return json.dumps(dict(format=FORMAT, normalization_profile=PROFILE,
                           payload_json=values[-1], seal_sha256=digest(framed)),
                      ensure_ascii=False).encode()


def adjudicate(result, accepted):
    if accepted:
        valid = (result.returncode == 0 and result.stdout == b'BASELINE_V1_ACCEPTED\n'
                 and result.stderr == b'')
    else:
        # A crash or arbitrary nonzero exit is not evidence of parser rejection.
        valid = (result.returncode == 1 and result.stdout == b''
                 and result.stderr.startswith(b'BASELINE_V1_REJECTED: missing field `sanitized_ingress`'))
    if not valid:
        raise RuntimeError(f'unexpected reader result: exit={result.returncode}, '
                           f'stdout={result.stdout!r}, stderr={result.stderr!r}')
    return 'accepted' if accepted else 'rejected_missing_sanitized_ingress'


def run(repo, fixture, output, repeats=2):
    if repeats < 2:
        raise RuntimeError('at least two clean builds required')
    if output.exists():
        raise RuntimeError('output already exists; preserve prior evidence')
    raw = fixture.read_bytes()
    require_hash(raw, FIXTURE, 'fixture')
    git = lambda *args: checked(['git', '-C', str(repo), *args])
    if git('rev-parse', BASELINE + '^{tree}').decode().strip() != TREE:
        raise RuntimeError('baseline tree mismatch')
    lock = git('show', BASELINE + ':Cargo.lock')
    require_hash(lock, LOCK, 'baseline lockfile')
    adapter = (Path(__file__).resolve().parent / 'baseline_reader.rs').read_bytes()
    # Fail before building if the compiler or Cargo is unavailable. No installer/fetch.
    rustc = checked(['rustc', '-vV']).decode()
    cargo = checked(['cargo', '-V']).decode().strip()
    if not rustc.startswith('rustc 1.97.1 ') or not cargo.startswith('cargo 1.97.1 '):
        raise RuntimeError('baseline characterization requires Rust/Cargo 1.97.1')
    sysroot = Path(checked(['rustc', '--print', 'sysroot']).decode().strip())
    suffix = '.exe' if os.name == 'nt' else ''
    tool_hashes = {name: digest((sysroot / 'bin' / (name + suffix)).read_bytes())
                   for name in ['rustc', 'cargo']}
    archive = git('archive', '--format=tar', BASELINE)
    report = dict(kind='source-pinned-baseline-characterization',
                  historical_release_certification=False, baseline_commit=BASELINE,
                  baseline_tree=TREE, baseline_lock_sha256=LOCK, fixture_sha256=FIXTURE,
                  adapter_sha256=digest(adapter), harness_sha256=digest(Path(__file__).read_bytes()),
                  rustc=rustc, cargo=cargo, tool_binary_sha256=tool_hashes,
                  source_archive_sha256=digest(archive), envelope_sha256=digest(envelope(raw)), runs=[])
    for _ in range(repeats):
        # Each build starts with fresh sources AND a fresh target. Only this context
        # manager's disposable directory is removed, including after a failed build.
        with tempfile.TemporaryDirectory(prefix='sovereign-baseline-reader-') as temp:
            source = Path(temp) / 'source'
            source.mkdir()
            with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
                tar.extractall(source, filter='data')
            example = source / 'crates/beta001-harness/examples/baseline_compat_reader.rs'
            example.parent.mkdir(exist_ok=True)
            example.write_bytes(adapter)
            env = os.environ.copy()
            env.update(CARGO_TARGET_DIR=str(Path(temp) / 'target'), CARGO_INCREMENTAL='0',
                       CARGO_PROFILE_DEV_DEBUG='0', CARGO_BUILD_JOBS='2', CARGO_NET_OFFLINE='true')
            command = ['cargo', 'build', '--offline', '--locked', '-p', 'beta001-harness',
                       '--example', 'baseline_compat_reader']
            checked(command, cwd=source, env=env)
            require_hash((source / 'Cargo.lock').read_bytes(), LOCK, 'post-build lockfile')
            executable = Path(env['CARGO_TARGET_DIR']) / 'debug/examples/baseline_compat_reader'
            if os.name == 'nt':
                executable = executable.with_suffix('.exe')
            outcomes = {}
            for name, data, accepted in [('raw_v1', raw, True),
                                          ('genuine_envelope', envelope(raw), False)]:
                result = subprocess.run([str(executable)], input=data,
                                        capture_output=True, check=False, timeout=30)
                outcomes[name] = adjudicate(result, accepted)
            report['runs'].append(dict(command=command, outcomes=outcomes,
                                       reader_binary_sha256=digest(executable.read_bytes())))
    if any(r['outcomes'] != report['runs'][0]['outcomes'] for r in report['runs']):
        raise RuntimeError('clean-build semantic results differ')
    report['semantic_repeatability'] = 'passed'
    with output.open('x', encoding='utf-8') as stream:
        json.dump(report, stream, indent=2)
        stream.write('\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    fixture = args.fixture or repo / 'crates/beta001-harness/tests/fixtures/unicode17_v1_package.json'
    try:
        run(repo, fixture, args.output)
    except (OSError, RuntimeError, tarfile.TarError, subprocess.TimeoutExpired) as error:
        parser.exit(1, f'baseline characterization FAILED: {error}\n')
    print(f'Baseline characterization passed; report: {args.output}')


if __name__ == '__main__':
    main()
