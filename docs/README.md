# Claude Relay Service - Documentation

Welcome to the Claude Relay Service documentation! This directory contains all project documentation organized by category.

## 📚 Quick Navigation

### User Guides
- [Quick Start](guides/quickstart.md) - Get started with the service
- [API Reference](guides/api-reference.md) - Complete API documentation
- [Deployment Guide](guides/deployment.md) - Production deployment instructions
- [Debugging Guide](guides/debugging.md) - Troubleshooting and debugging tips
- [Troubleshooting Guide](guides/troubleshooting.md) - Common issues and solutions

### Architecture & Technical Documentation
- [Architecture Overview](architecture/overview.md) - System architecture and design
- [API Interfaces](architecture/interfaces.md) - Interface specifications
- [Security](architecture/security.md) - Security audit and best practices
- [Testing](architecture/testing.md) - Comprehensive testing documentation
- [Redis Data Schema](architecture/redis-schema.md) - Redis key patterns and data structures

### Development
- [Contributing](development/contributing.md) - How to contribute to this project
- [Roadmap](development/roadmap.md) - Project roadmap and future plans
- [CLI Usage Guide](development/cli-usage.md) - Command-line tools reference
- [Troubleshooting](development/troubleshooting/)
  - [Environment Setup](development/troubleshooting/environment.md)

### Archives
- [Migration Documentation](archive/migration/) - Node.js to Rust migration history
  - [Migration Guide](archive/MIGRATION_GUIDE.md)
  - [Migration Plan](archive/migration/RUST_MIGRATION_PLAN.md)
  - [Migration Summary](archive/migration/FINAL_MIGRATION_SUMMARY.md)
  - [Refactoring Status](archive/migration/REFACTORING_STATUS.md)
  - [Progress Log](archive/migration/PROGRESS.md)
- [Phase Completion Reports](archive/phases/) - Historical development phase documentation
  - 20 phase/day/week completion reports

## 🗂️ Documentation Organization

```
docs/
├── README.md (this file)
├── guides/              # User guides and reference material
├── architecture/        # Technical architecture documentation
├── development/         # Developer resources
└── archive/             # Historical documentation
    ├── phases/          # Development phase reports
    └── migration/       # Migration documentation
```

## 🔍 Finding What You Need

- **New Users**: Start with [Quick Start](guides/quickstart.md)
- **API Integration**: See [API Reference](guides/api-reference.md)
- **Deployment**: Check [Deployment Guide](guides/deployment.md)
- **Development**: Review [Contributing](development/contributing.md) and [Roadmap](development/roadmap.md)
- **Testing**: Check [Testing Documentation](architecture/testing.md)
- **Troubleshooting**: See [Troubleshooting Guide](guides/troubleshooting.md)
- **CLI Tools**: Reference [CLI Usage Guide](development/cli-usage.md)
- **Redis Schema**: Understand data structures in [Redis Schema](architecture/redis-schema.md)
- **Historical Context**: Explore [Archives](archive/)

## 📝 Documentation Standards

- All documentation uses Markdown format
- Code examples should be syntax-highlighted
- Include table of contents for long documents
- Keep documentation up-to-date with code changes
- Archive outdated documentation rather than deleting it

## 🤝 Contributing to Documentation

See [Contributing Guide](development/contributing.md) for information on improving documentation.

## 📜 Changelog

**2025-11-01 - Phase 5 CLAUDE.md Optimization:**
- Extracted [Troubleshooting Guide](guides/troubleshooting.md) from CLAUDE.md (410 lines)
- Extracted [CLI Usage Guide](development/cli-usage.md) from CLAUDE.md (645 lines)
- Extracted [Redis Schema](architecture/redis-schema.md) from CLAUDE.md (915 lines)
- Streamlined CLAUDE.md from 614 to 507 lines (17% reduction)
- Added comprehensive documentation index to CLAUDE.md

**2025-11-01 - Phase 4 Documentation Consolidation:**
- Merged 3 quickstart guides into single comprehensive [Quick Start](guides/quickstart.md)
- Merged 8 testing documents into unified [Testing Documentation](architecture/testing.md)
- Merged 2 roadmap documents into consolidated [Project Roadmap](development/roadmap.md)
- Created project [CHANGELOG.md](../CHANGELOG.md) with migration history
- Updated documentation links and removed obsolete "(coming soon)" notes

**2025-01-01 - Major Reorganization:**
- Migrated from fragmented structure to organized hierarchy
- Archived 25 historical phase/migration documents
- Consolidated technical guides under `guides/` and `architecture/`
- Created comprehensive documentation index

---

*Last updated: 2025-11-01*
