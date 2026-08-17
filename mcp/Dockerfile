# Claunote MCP server — companion container that exposes your notes
# to Claude over the Model Context Protocol. Meant to run reachable
# from the internet, so it enforces a mandatory bearer token itself
# (see server.js) in addition to whatever sits in front of it.

# ---- Build stage: resolve node_modules only ----
FROM node:22-alpine AS build
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --omit=dev

# ---- Final stage: no npm CLI at all ----
# The npm CLI ships its own bundled dependency tree (tar, sigstore,
# picomatch, brace-expansion...) that has nothing to do with this
# app's actual dependencies but still shows up in vulnerability scans
# as long as npm itself is present. This app only ever runs
# `node server.js` - npm/npx are never invoked after the build stage -
# so the final image drops them entirely instead of carrying that
# tooling's CVEs for no reason.
FROM node:22-alpine
WORKDIR /app

# Keep OS packages patched at build time, mirroring the main image.
RUN apk upgrade --no-cache \
    && rm -rf /usr/local/lib/node_modules/npm /usr/local/bin/npm /usr/local/bin/npx /opt/yarn*

COPY --from=build /app/node_modules ./node_modules
COPY package.json server.js ./

USER node
EXPOSE 4200

HEALTHCHECK --interval=60s --timeout=5s --retries=3 \
  CMD wget -qO- http://127.0.0.1:4200/healthz || exit 1

CMD ["node", "server.js"]
