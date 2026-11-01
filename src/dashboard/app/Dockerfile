FROM node:24-alpine@sha256:c921b97d4b74f51744057454b306b418cf693865e73b8100559189605f6955b8

# Install prettier globally
RUN npm install -g prettier

# Workdir inside container
WORKDIR /app

# Default command: show prettier help
CMD ["prettier", "--help"]

