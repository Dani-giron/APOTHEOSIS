# Architectural Documentation: APOTHEOSIS2

Framework: **Views & Beyond** (Clements et al.)

**Start here:** read [roadmap.md](roadmap.md) first. It describes what each document covers, which document to read depending on your role, and how the views relate to each other.

## Views

| View | File | Status |
|-------|---------|--------|
| Module | [views/module/vista-modulos.md](views/module/vista-modulos.md) | Complete |
| Component and Connector (C&C) | [views/cc/vista-cc.md](views/cc/vista-cc.md) | Complete |
| Deployment | [views/deployment/vista-distribucion.md](views/deployment/vista-distribucion.md) | Complete |

## Cross-cutting Documents

- [Roadmap](roadmap.md)
- [Quality Attributes](quality-attributes.md): design drivers, out-of-scope attributes, and mapping to the three views
- [Directory and Data Dictionary](directory.md): glossary, acronyms, central type catalog, and bibliographic references

## Folder Structure

```
docs/architecture/
├── README.md                        ← this file
├── roadmap.md
├── quality-attributes.md             ← quality attributes: drivers, trade-offs, and cross-view mapping
├── directory.md                     ← glossary, acronyms, data dictionary, and references
└── views/
    ├── module/
    │   ├── vista-modulos.md
    │   └── assets/                  ← PNGs/SVGs of the module diagram
    ├── cc/
    │   ├── vista-cc.md
    │   └── assets/
    └── deployment/
        ├── vista-distribucion.md
        └── assets/
```
