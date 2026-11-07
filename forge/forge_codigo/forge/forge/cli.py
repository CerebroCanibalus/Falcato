"""
Entrypoint de la CLI. Solo parsea argumentos y delega a commands/.
Sin lógica de negocio aquí — así la futura GUI puede importar
directamente forge.commands sin pasar por esta capa.
"""
import argparse
import sys

from .commands import build, clean, new, watch


def main() -> int:
    parser = argparse.ArgumentParser(prog="forge", description="Automatización para proyectos Falcato")
    sub = parser.add_subparsers(dest="comando", required=True)

    p_build = sub.add_parser("build", help="Compila el proyecto")
    p_build.add_argument("archivo", nargs="?", help="Archivo .fc a compilar (opcional, usa entry de forge.toml)")

    p_watch = sub.add_parser("watch", help="Recompila automáticamente al detectar cambios")
    p_watch.add_argument("archivo", nargs="?")

    p_new = sub.add_parser("new", help="Crea un proyecto Falcato nuevo")
    p_new.add_argument("nombre")

    sub.add_parser("clean", help="Borra artefactos de compilación")

    args = parser.parse_args()

    if args.comando == "build":
        ok = build.ejecutar(args.archivo)
        return 0 if ok else 1
    elif args.comando == "watch":
        watch.ejecutar(args.archivo)
        return 0
    elif args.comando == "new":
        ok = new.ejecutar(args.nombre)
        return 0 if ok else 1
    elif args.comando == "clean":
        ok = clean.ejecutar()
        return 0 if ok else 1

    return 1


if __name__ == "__main__":
    sys.exit(main())
