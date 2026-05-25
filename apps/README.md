# Nodalix Native Apps

`apps/` contains the native and first-party application ecosystem for Nodalix OS.

The long-term goal is a coherent Linux-native desktop suite: system shell components, productivity tools, media tools, engineering tools, and developer tooling that share one visual language and safety model.

## Categories

- System apps: shell, settings, files, notifications, updates, capture, power, network, audio, Bluetooth, and OS onboarding.
- Productivity apps: documents, spreadsheets, presentations, notes, drawing, and local transfer.
- Engineering apps: CAD, scientific tools, plotting, unit conversion, airfoil, structures, and CFD research.
- Media apps: photo and video tools.
- Developer apps: Nodalix DevKit and Bundle Creator.

## Development conventions

- Each app should have `README.md`, `ROADMAP.md`, `SPEC.md`, and `STATUS.md`.
- Rust + GTK4/libadwaita is preferred for native system apps.
- Tauri is acceptable where already established, such as `nodalix-files`.
- Prototypes must not modify system configuration automatically.
- Destructive actions require confirmation.
- Wrappers live in `local/bin`.
- Shared OS configuration lives in `config/`.
- Build artifacts such as `target/`, `node_modules/`, `dist/`, and `build/` are not part of the source contract.

## Planned ecosystem

```text
apps/
├─ native-app-template
├─ nodalix-greeter
├─ nodalix-lock
├─ nodalix-settings
├─ nodalix-control-center
├─ nodalix-bar
├─ nodalix-dock
├─ nodalix-command-bar
├─ nodalix-files
├─ nodalix-notifications
├─ nodalix-wallpapers
├─ nodalix-welcome
├─ nodalix-updater
├─ nodalix-store
├─ nodalix-monitor
├─ nodalix-capture
├─ nodalix-audio
├─ nodalix-network
├─ nodalix-bluetooth
├─ nodalix-power
├─ nodalix-tweaks
├─ nodalix-bundle-creator
├─ nodalix-pages
├─ nodalix-cells
├─ nodalix-point
├─ nodalix-drop
├─ nodalix-photo
├─ nodalix-video
├─ nodalix-draw
├─ nodalix-notes
├─ nodalix-cad
├─ nodalix-lab
├─ nodalix-plot
├─ nodalix-units
├─ nodalix-airfoil
├─ nodalix-structures
├─ nodalix-cfd
└─ nodalix-devkit
```

## Current implementation model

Existing apps are preserved. New apps start as documented prototypes or minimal Rust/GTK skeletons. Nothing in this directory should replace Waybar, modify Hyprland autostart, edit greetd, or install services without an explicit integration step.

# Lix Apps — Ecosistema nativo de Nodalix OS

Este documento define una propuesta unificada para las aplicaciones propias de **Nodalix OS**, agrupando herramientas equivalentes de Windows, Adobe, Autodesk, MATLAB, Office y software científico/ingenieril en una suite coherente bajo la marca **Lix**.

La idea no es crear una app por cada programa existente, sino crear **apps Lix potentes por categoría**, con módulos internos.

---

## Principio de diseño

En vez de tener decenas de aplicaciones separadas:

- Adobe Premiere
- DaVinci Resolve
- After Effects
- Media Encoder

se agrupan en una sola app:

- **LixStudio**

Y dentro de ella habría módulos:

- Edición de vídeo
- Color
- Motion graphics
- VFX
- Render/export

Este mismo criterio se aplica al resto del ecosistema.

---


# Lix Apps — Ecosistema nativo de Nodalix OS

Este documento define una propuesta refinada para las aplicaciones propias de **Nodalix OS**, agrupando herramientas equivalentes de Windows, Adobe, Autodesk, MATLAB, Office y software científico/ingenieril en una suite coherente bajo la marca **Lix**.

La idea no es crear una app por cada programa existente, sino crear **apps Lix potentes por categoría**, con nombres claros, memorables y con identidad propia.

---

## Principio de diseño

En vez de tener decenas de aplicaciones separadas como:

- Adobe Premiere
- DaVinci Resolve
- After Effects
- Media Encoder

se agrupan en una sola app:

- **LixStudio**

Y dentro de ella habría módulos:

- Edición de vídeo
- Color
- Motion graphics
- VFX
- Render/export

Este mismo criterio se aplica al resto del ecosistema.

---

## Criterio de nombres

Los nombres deben ser:

- fáciles de recordar,
- cortos,
- reconocibles,
- profesionales,
- visuales cuando sea posible,
- coherentes con la marca Lix,
- y suficientemente amplios para permitir módulos internos.

---

# 1. Sistema Nodalix

| App Lix | Sustituye / agrupa | Función |
|---|---|---|
| **LixFiles** | Windows Explorer, Finder, Nautilus, Dolphin | Gestor de archivos nativo |
| **LixSettings** | Windows Settings, GNOME Settings, KDE Settings | Ajustes generales del sistema |
| **LixControl** | macOS Control Center, Windows Quick Settings | Centro rápido de controles del sistema |
| **LixStore** | Microsoft Store, App Store, Discover | Tienda e instalador de apps |
| **LixUpdate** | Windows Update, Discover Updates | Actualizaciones del sistema |
| **LixShield** | Windows Defender, firewall GUIs | Seguridad, firewall y privacidad |
| **LixDrop** | AirDrop, LocalSend, Phone Link | Transferencia rápida de archivos |
| **LixShot** | Snipping Tool, ShareX, OBS básico | Capturas y grabación rápida |
| **LixWelcome** | Welcome apps, setup assistants | Bienvenida y configuración inicial |
| **LixGreeter** | SDDM, GDM, Windows Login | Pantalla de inicio de sesión |
| **LixLock** | Windows Lock Screen, Hyprlock | Pantalla de bloqueo |
| **LixBar** | Waybar, taskbar, top bar | Barra superior del sistema |
| **LixDock** | macOS Dock, taskbar | Dock de aplicaciones |
| **LixNotify** | Notification Center | Centro de notificaciones |
| **LixTerm** | Windows Terminal, Ghostty, Alacritty | Terminal nativa |
| **LixTune** | PowerToys, herramientas avanzadas | Optimización y ajustes avanzados |

---

# 2. Productividad y documentación

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixSuite** | Microsoft Office, LibreOffice, iWork, Google Docs | Hub general de productividad |
| **LixPages** | Microsoft Word, Apple Pages, LibreOffice Writer | Documentos de texto |
| **LixCells** | Microsoft Excel, Apple Numbers, LibreOffice Calc | Hojas de cálculo |
| **LixDeck** | PowerPoint, Keynote, LibreOffice Impress | Presentaciones |
| **LixPaper** | Adobe Acrobat, PDF-XChange, Preview | PDF, documentos, firmas y escáner |
| **LixNotes** | OneNote, Notion, Obsidian, Apple Notes | Notas y base de conocimiento |
| **LixRefs** | Zotero, Mendeley, EndNote | Bibliografía y citas |
| **LixPlan** | Microsoft Project, Planner, Trello, Jira básico | Gestión de proyectos |
| **LixFlow** | Microsoft Visio, Draw.io, Lucidchart | Diagramas, flujos y esquemas |
| **LixBoard** | Miro, Microsoft Whiteboard | Pizarra visual |
| **LixWrite** | Grammarly, LanguageTool, asistentes de escritura | Corrección, escritura y asistencia textual |

---

# 3. Creatividad, multimedia y diseño

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixStudio** | Adobe Premiere, DaVinci Resolve, After Effects, Media Encoder | Vídeo, color, VFX, motion y render |
| **LixPhoto** | Photoshop, Lightroom, Affinity Photo, Camera RAW | Foto, RAW, retoque y biblioteca |
| **LixCanvas** | Illustrator, Figma, Adobe XD, Affinity Designer, Canva | Vectorial, UI/UX, iconos, prototipos y diseño rápido |
| **LixPress** | InDesign, Publisher, Scribus | Maquetación editorial y publicaciones |
| **LixSound** | Audition, Audacity, Logic básico | Grabación, edición y mezcla de audio |
| **LixForge** | Blender, Cinema4D, Substance Painter | Modelado 3D creativo, materiales, render y animación |
| **LixMotion** | Integrado en LixStudio | Motion graphics y animación |
| **LixCreate** | Integrado en LixCanvas | Diseño rápido para documentos y redes |

---

# 4. CAD, modelado e ingeniería gráfica

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixCAD** | AutoCAD, BricsCAD, DraftSight, LibreCAD | CAD 2D/3D general, DWG, DXF, STEP |
| **LixModel** | Fusion 360, Inventor, SolidWorks, Creo, FreeCAD | Modelado paramétrico mecánico |
| **LixBuild** | Revit, Archicad, Navisworks | Arquitectura, BIM, construcción y coordinación |
| **LixTerrain** | Civil 3D, InfraWorks, Global Mapper civil | Obra civil, terreno, carreteras y topografía |
| **LixFrame** | Tekla, Robot, SAP2000, CYPE estructuras | Estructuras metálicas y hormigón |
| **LixSurface** | Rhino, Grasshopper, Alias | Superficies, NURBS y diseño paramétrico |
| **LixDraft** | Integrado en LixCAD | Modo de dibujo técnico 2D ligero |
| **LixMesh** | HyperMesh, Salome, Gmsh | Mallado y preprocesado CAE |

---

# 5. Matemáticas, ciencia y simulación

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixMath** | MATLAB, Mathematica, Maple, Mathcad, Octave | Cálculo numérico, simbólico, notebooks y scripts |
| **LixSim** | Simulink, LabVIEW visual, Modelica | Simulación por bloques y sistemas dinámicos |
| **LixGraph** | OriginPro, Desmos, GeoGebra, GraphPad Prism | Gráficas, análisis de datos y ajuste de curvas |
| **LixLab** | LabVIEW, ELN, instrumentación | Laboratorio, adquisición de datos y cuaderno técnico |
| **LixUnits** | Conversores de unidades, tablas técnicas | Unidades, constantes y propiedades físicas |
| **LixBook** | Jupyter, Spyder, RStudio | Notebooks científicos Python/R/Julia |
| **LixData** | Power BI, Tableau, Orange, Weka | BI, datasets y machine learning visual |

---

# 6. CAE, CFD, FEA y aeroespacial

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixCAE** | ANSYS Workbench, Abaqus, Nastran, COMSOL | Simulación general CAE y multifísica |
| **LixCFD** | ANSYS Fluent, CFX, OpenFOAM, SU2 | Dinámica de fluidos computacional |
| **LixStress** | ANSYS Mechanical, Abaqus, Nastran | Elementos finitos estructurales |
| **LixAero** | XFLR5, AVL, SU2 Aero | Aeronaves, alas, estabilidad y polares |
| **LixFoil** | XFOIL, JavaFoil, Profili | Perfiles aerodinámicos |
| **LixMotion** | Adams, LS-DYNA, dinámica multicuerpo | Dinámica, impacto y sistemas multicuerpo |
| **LixPrePost** | Integrado en LixCAE | Preprocesado y postprocesado |

> Nota: **LixMotion** puede tener dos módulos: motion graphics dentro de **LixStudio** y dinámica/multicuerpo dentro de **LixCAE**. Si quieres evitar conflicto total, usa **LixDynamics** para ingeniería y deja **LixMotion** para creatividad.

---

# 7. Electrónica, control e IoT

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixCircuit** | KiCad, Altium, Proteus, Multisim, LTspice | PCB, esquemas y simulación electrónica |
| **LixAutomate** | TIA Portal, Codesys, Simulink Control | PLC, control automático y automatización |
| **LixRobotics** | ROS tools, Gazebo, RViz | Robótica, sensores y simulación |
| **LixIoT** | Home Assistant tools, Node-RED, MQTT tools | IoT, domótica y automatización |
| **LixElectrical** | AutoCAD Electrical, EPLAN | Esquemas eléctricos industriales |

> Nota: **LixControl** queda reservado para el centro de control del sistema. Para control automático/PLC es mejor **LixAutomate**, así no hay conflicto de nombres.

---

# 8. Desarrollo, datos e inteligencia artificial

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixCode** | VS Code, Zed, JetBrains, Visual Studio | Editor/IDE |
| **LixDev** | GitHub Desktop, Docker Desktop, Postman, DBeaver | Git, contenedores, APIs y bases de datos |
| **LixAPI** | Postman, Insomnia | APIs y peticiones HTTP |
| **LixDB** | DBeaver, DataGrip, pgAdmin | Bases de datos |
| **LixGit** | GitHub Desktop, GitKraken | Cliente Git visual |
| **LixContainers** | Docker Desktop, Podman Desktop | Contenedores |
| **LixAI** | Ollama GUI, OpenAI tools, LM Studio | IA local/cloud, modelos y agentes |
| **LixML** | Weka, Orange, herramientas ML | Machine learning visual |

---

# 9. GIS, topografía, territorio e hidráulica

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixGIS** | ArcGIS, QGIS, Global Mapper | GIS, mapas, capas y análisis espacial |
| **LixTerrain** | Civil 3D, InfraWorks, DEM tools | Terreno, topografía, curvas de nivel y nubes de puntos |
| **LixHydro** | HEC-RAS, EPANET, SWMM | Hidráulica, redes de agua y drenaje |
| **LixMaps** | QGIS ligero, mapas técnicos | Mapas y visualización territorial |
| **LixInfra** | InfraWorks, Civil 3D infraestructuras | Infraestructura 3D |

---

# 10. Química, biología y laboratorio científico

| App Lix | Sustituye / agrupa | Módulos internos |
|---|---|---|
| **LixChem** | ChemDraw, Avogadro, Gaussian UI | Química, moléculas y estructuras |
| **LixBio** | PyMOL, ImageJ, Fiji | Biología, imagen científica y visualización molecular |
| **LixPrism** | GraphPad Prism, Origin bioestadística | Bioestadística y gráficas científicas |
| **LixQuantum** | Gaussian, GaussView, química computacional | Química cuántica y simulación molecular |
| **LixMolecule** | Avogadro, PyMOL | Modelado molecular |

---

# 11. Cambios de naming aplicados

| Nombre actual | Nuevo nombre | Motivo |
|---|---|---|
| LixControl Center | **LixControl** | Más corto y recordable; el “Center” sobra |
| LixUpdater | **LixUpdate** | Más limpio, estilo app de sistema |
| LixNotifications | **LixNotify** | Más corto y con identidad |
| LixTerminal | **LixTerm** | Más técnico y corto |
| LixTweaks | **LixTune** | “Tune” suena a optimizar/ajustar fino |
| LixOffice | **LixSuite** | Más amplio que Office |
| LixNumbers | **LixCells** | Más reconocible como hoja de cálculo |
| LixSlides | **LixDeck** | Más memorable y moderno |
| LixPDF | **LixPaper** | Más elegante; PDF + documentos + firma + escáner |
| LixCite | **LixRefs** | Más claro para referencias/bibliografía |
| LixProject | **LixPlan** | Más simple y amplio |
| LixDiagram | **LixFlow** | Más bonito para diagramas, flujos y esquemas |
| LixProof | **LixWrite** | Mejor para corrección, escritura y asistencia |
| LixDesign | **LixCanvas** | Más visual y memorable para vectorial/UI/UX |
| LixLayout | **LixPress** | Suena editorial/profesional |
| LixAudio | **LixSound** | Más de producto final, menos genérico |
| Lix3D | **LixForge** | Más potente para 3D/modelado/render |
| LixAnimate | **Integrado en LixStudio** | No hace falta app separada al principio |
| LixCreate | **Integrado en LixCanvas** | Diseño rápido dentro de la app de diseño |
| LixModel | **LixModel** | Se mantiene para CAD paramétrico |
| LixBIM | **LixBuild** | Más memorable para arquitectura/BIM/construcción |
| LixCivil | **LixTerrain** | Mejor si incluye civil, topografía y terreno |
| LixStructures | **LixFrame** | Más corto y visual |
| LixDraft | **Integrado en LixCAD** | Sería modo Draft dentro de LixCAD |
| LixPlot | **LixGraph** | Graph suena más serio y científico |
| LixNotebook | **LixBook** | Más elegante, aunque Notebook es más explícito |
| LixFEA | **LixStress** | Más recordable para estructuras/esfuerzos |
| LixAirfoil | **LixFoil** | Más corto y memorable |
| LixPrePost | **Integrado en LixCAE** | No hace falta app separada al principio |
| LixControlSim | **LixAutomate** | Evita conflicto con LixControl del sistema |

---

# 12. Apps principales recomendadas para empezar

## Sistema

| Prioridad | App |
|---:|---|
| 1 | **LixFiles** |
| 2 | **LixSettings** |
| 3 | **LixControl** |
| 4 | **LixBar** |
| 5 | **LixGreeter** |
| 6 | **LixLock** |
| 7 | **LixDock** |
| 8 | **LixStore** |
| 9 | **LixUpdate** |
| 10 | **LixDrop** |

## Productividad

| Prioridad | App |
|---:|---|
| 11 | **LixSuite** |
| 12 | **LixPages** |
| 13 | **LixCells** |
| 14 | **LixDeck** |
| 15 | **LixPaper** |
| 16 | **LixNotes** |
| 17 | **LixRefs** |
| 18 | **LixFlow** |

## Creatividad

| Prioridad | App |
|---:|---|
| 19 | **LixStudio** |
| 20 | **LixPhoto** |
| 21 | **LixCanvas** |
| 22 | **LixSound** |
| 23 | **LixForge** |
| 24 | **LixShot** |

## Ingeniería y ciencia

| Prioridad | App |
|---:|---|
| 25 | **LixCAD** |
| 26 | **LixModel** |
| 27 | **LixMath** |
| 28 | **LixSim** |
| 29 | **LixGraph** |
| 30 | **LixCFD** |
| 31 | **LixFrame** |
| 32 | **LixFoil** |
| 33 | **LixUnits** |
| 34 | **LixCAE** |
| 35 | **LixStress** |

## Desarrollo y datos

| Prioridad | App |
|---:|---|
| 36 | **LixCode** |
| 37 | **LixDev** |
| 38 | **LixBook** |
| 39 | **LixData** |
| 40 | **LixAI** |

---

# 13. Núcleo mínimo realista de Nodalix OS

Si el objetivo es tener una primera versión coherente y usable, el núcleo mínimo debería ser:

| Categoría | Apps |
|---|---|
| Sistema | **LixFiles**, **LixSettings**, **LixControl**, **LixBar**, **LixGreeter**, **LixLock** |
| Productividad | **LixPages**, **LixCells**, **LixDeck**, **LixPaper**, **LixNotes** |
| Creatividad | **LixPhoto**, **LixStudio**, **LixCanvas** |
| Ingeniería | **LixCAD**, **LixMath**, **LixSim**, **LixGraph**, **LixUnits** |
| Desarrollo | **LixCode**, **LixDev** |
| Utilidades | **LixDrop**, **LixShot**, **LixStore**, **LixUpdate** |

Total aproximado inicial: **24 apps**.

---

# 14. Naming final recomendado

## Sistema

- **LixFiles**
- **LixSettings**
- **LixControl**
- **LixBar**
- **LixDock**
- **LixGreeter**
- **LixLock**
- **LixStore**
- **LixUpdate**
- **LixShield**
- **LixDrop**
- **LixShot**
- **LixNotify**
- **LixTerm**
- **LixTune**

## Productividad

- **LixSuite**
- **LixPages**
- **LixCells**
- **LixDeck**
- **LixPaper**
- **LixNotes**
- **LixRefs**
- **LixPlan**
- **LixFlow**
- **LixBoard**
- **LixWrite**

## Creatividad

- **LixStudio**
- **LixPhoto**
- **LixCanvas**
- **LixPress**
- **LixSound**
- **LixForge**

## Ingeniería

- **LixCAD**
- **LixModel**
- **LixBuild**
- **LixTerrain**
- **LixFrame**
- **LixSurface**
- **LixMesh**

## Ciencia y simulación

- **LixMath**
- **LixSim**
- **LixGraph**
- **LixLab**
- **LixUnits**
- **LixBook**
- **LixData**

## CAE y aeroespacial

- **LixCAE**
- **LixCFD**
- **LixStress**
- **LixAero**
- **LixFoil**
- **LixDynamics**

## Electrónica y automatización

- **LixCircuit**
- **LixAutomate**
- **LixRobotics**
- **LixIoT**
- **LixElectrical**

## Desarrollo e IA

- **LixCode**
- **LixDev**
- **LixAPI**
- **LixDB**
- **LixGit**
- **LixContainers**
- **LixAI**
- **LixML**

## Territorio

- **LixGIS**
- **LixTerrain**
- **LixHydro**
- **LixMaps**
- **LixInfra**

## Ciencia bio/química

- **LixChem**
- **LixBio**
- **LixPrism**
- **LixQuantum**
- **LixMolecule**

---

# 15. Filosofía final

Nodalix OS no debería intentar parecer una colección de clones.

La filosofía debería ser:

> Una suite nativa, coherente y elegante para Linux, enfocada en productividad, creatividad, ingeniería y ciencia.

Cada app Lix debe:

- tener identidad visual propia,
- compartir diseño común,
- integrarse con Nodalix OS,
- evitar depender de interfaces antiguas,
- priorizar flujos reales de trabajo,
- ser honesta con sus limitaciones,
- y crecer por módulos internos en lugar de multiplicar apps innecesarias.

---
