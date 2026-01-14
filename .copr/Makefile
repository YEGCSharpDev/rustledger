.PHONY: srpm

# Get the directory where this Makefile lives
MAKEFILE_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
# Root of the repo is one level up
REPO_ROOT := $(abspath $(MAKEFILE_DIR)/..)
SPECFILE := $(REPO_ROOT)/packaging/rpm/rustledger.spec

srpm:
	dnf install -y rpmdevtools rpm-build
	spectool -g -C $(outdir) $(SPECFILE)
	rpmbuild -bs $(SPECFILE) \
		--define "_sourcedir $(outdir)" \
		--define "_srcrpmdir $(outdir)"
