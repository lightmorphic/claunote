# Claunote MCP server — companion container that exposes your notes
# to Claude over the Model Context Protocol. Unlike Notespice's
# optional/off-by-default version, this one is meant to run reachable
# from the internet, so it enforces a mandatory bearer token itself
# (see server.js) in addition to whatever sits in front of it.
FROM node:22-alpine

# Keep OS packages patched at build time, mirroring the main image.
RUN apk upgrade --no-cache

WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --omit=dev && npm cache clean --force
COPY server.js ./

USER node
EXPOSE 4200

HEALTHCHECK --interval=60s --timeout=5s --retries=3 \
  CMD wget -qO- http://127.0.0.1:4200/healthz || exit 1

CMD ["node", "server.js"]
