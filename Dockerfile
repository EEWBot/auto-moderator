FROM rust:1.98.0 as build-env
LABEL maintainer="yanorei32"

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

WORKDIR /usr/src
RUN cargo new auto-moderator
COPY LICENSE Cargo.toml Cargo.lock /usr/src/auto-moderator/
WORKDIR /usr/src/auto-moderator
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN	cargo install cargo-license && cargo license \
	--authors \
	--do-not-bundle \
	--avoid-dev-deps \
	--avoid-build-deps \
	--filter-platform "$(rustc -vV | sed -n 's|host: ||p')" \
	> CREDITS

RUN cargo build --release
COPY src/ /usr/src/auto-moderator/src/

RUN touch src/* && cargo build --release

FROM debian:trixie-slim

WORKDIR /

COPY --chown=root:root --from=build-env \
	/usr/src/auto-moderator/CREDITS \
	/usr/src/auto-moderator/LICENSE \
	/usr/share/licenses/auto-moderator/

COPY --chown=root:root --from=build-env \
	/usr/src/auto-moderator/target/release/auto-moderator \
	/usr/bin/auto-moderator

CMD ["/usr/bin/auto-moderator"]
