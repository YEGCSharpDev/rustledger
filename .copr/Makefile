.PHONY: srpm

# Get the directory where this Makefile lives
MAKEFILE_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
# Root of the repo is one level up
REPO_ROOT := $(abspath $(MAKEFILE_DIR)/..)
SPECFILE := $(REPO_ROOT)/packaging/rpm/rustledger.spec

srpm:
	dnf install -y rpm-build git

	# Get version from latest git tag (v1.0.0-rc.18 -> 1.0.0-rc.18)
	cd $(REPO_ROOT) && \
	VERSION=$$(git describe --tags --abbrev=0 2>/dev/null | sed 's/^v//') && \
	RPM_VERSION=$$(echo $$VERSION | sed 's/-/~/g') && \
	echo "Building version $$VERSION (RPM: $$RPM_VERSION)" && \
	\
	# Create source tarball from git \
	git archive --format=tar.gz --prefix=rustledger-$$VERSION/ HEAD > $(outdir)/v$$VERSION.tar.gz && \
	\
	# Update spec file with correct version \
	sed -i "s/^Version:.*/Version:        $$RPM_VERSION/" $(SPECFILE) && \
	sed -i "s/%global version_upstream.*/%global version_upstream $$VERSION/" $(SPECFILE) && \
	\
	# Build SRPM \
	rpmbuild -bs $(SPECFILE) \
		--define "_sourcedir $(outdir)" \
		--define "_srcrpmdir $(outdir)" \
		--define "version_upstream $$VERSION"
