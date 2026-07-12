WASM_SRC  := core
WEB_DIR   := web
DIST_DIR  := $(WEB_DIR)/dist
PKG_DIR   := $(WEB_DIR)/pkg
GH_BRANCH := gh-pages
TMP_DEPLOY:= /tmp/orient-stl-deploy

.PHONY: all build wasm dev deploy commit test type-check clean

all: build

# ─── Build ────────────────────────────────────────────────────

wasm:
	wasm-pack build --target bundler --out-dir ../$(PKG_DIR) $(WASM_SRC)

build: wasm
	cd $(WEB_DIR) && npm run build

# ─── Dev ──────────────────────────────────────────────────────

dev:
	cd $(WEB_DIR) && npm run dev

# ─── Deploy ───────────────────────────────────────────────────

deploy: build
	rm -rf $(TMP_DEPLOY)
	mkdir -p $(TMP_DEPLOY)
	cp -r $(DIST_DIR)/. $(TMP_DEPLOY)/
	cp $(WEB_DIR)/public/favicon.svg $(TMP_DEPLOY)/ 2>/dev/null || true
	test -z "$$(git status --porcelain)" || git stash push -m "deploy-auto-stash"
	git checkout $(GH_BRANCH)
	rm -rf .nojekyll assets favicon.svg index.html 2>/dev/null; true
	cp -r $(TMP_DEPLOY)/. .
	touch .nojekyll
	git add assets/ index.html favicon.svg .nojekyll
	git commit -m "Deploy to gh-pages" --allow-empty
	git push origin $(GH_BRANCH)
	git checkout -
	git stash pop 2>/dev/null || true

# ─── Commit ───────────────────────────────────────────────────

commit:
	git add -A
	git commit -m "$(MESSAGE)" --allow-empty

# ─── Tests / Checks ──────────────────────────────────────────

test:
	cd $(WEB_DIR) && npm run test

type-check:
	cd $(WEB_DIR) && npm run type-check

# ─── Clean ────────────────────────────────────────────────────

clean:
	rm -rf $(DIST_DIR)
	rm -rf $(PKG_DIR)
