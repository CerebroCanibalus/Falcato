# 02 — Tu primer programa (y el secreto de la memoria)

← [01: Que es Falcato?](01-que-es-falcato.md) | [Indice](../GUIA.md) | [Siguiente: Variables →](03-variables.md)

---

Crea un archivo `hola.fc` en tu computadora. Puede estar en el escritorio, en una carpeta, el nombre da igual, solo que termine en `.fc`. Abre ese archivo y escribe **exactamente** esto:

```
función principal() -> Entero32 {
    decir("hola, mundo");
    retornar 0;
}
```

## El gran secreto que nadie te cuenta

Siento que te sientes frente a la pantalla y todo parezca magia. Pero no lo es. Tu computadora, tu teléfono, cualquier dispositivo con chip: en su interior hay un espacio de memoria. Piensa no en algo abstracto, piensa en **un almacén**.

Ese almacén tiene casilleros. Miles de ellos. Cada casillero:
- Tiene un número que lo identifica (la computadora lo recuerda, tú no necesitas).
- Puede guardar **una sola cosa**: un número entero, una letra, la palabra "hola", o estar completamente vacío.

```
Casillero 5 → [ 42 ]
Casillero 6 → [ 'A' ]
Casillero 7 → [ true ]
Casillero 8 → [   vacío   ]
Casillero 9 → [ 3.14159 ]
```

Un programa de computadora no es más que un conjunto de instrucciones para el encargado de ese almacén, ese "encargado" es Falcato, que se trata de un compilador, en el momento en que ejecuta tu código, es quien toma tus instrucciones y mueve las cosas de un casillero a otro, las muestra en la pantalla o las modifica en tiempo real. Cuando escribes un programa, estás escribiendo pasos para que ese encargado sepa qué hacer y el código es simplemente el idioma que usas para hablar con él.

## ¿Por qué ese orden?

Ahora mira el código que escribiste, seguro piensas "¿Por qué mierda empiezo con esa línea rara de `función principal`? ¿Por qué no va `hola, mundo`? ¿Por qué termina con `retornar 0`? ¿No sería más lógico empezar por lo que quieres mostrar y después preocuparse de devolver un número?".
Cuando alguien ve código por primera vez, ese orden parece arbitrario, hasta molesto (Y lo es). Pero hay una razón sencilla para hacerlo así:

**Primero, el nombre y la promesa.** La línea `función principal() -> Entero32 {` no es más que presentarte al encargado lo que vas a hacer, le estás diciendo que vas a hacer *algo* llamado `principal`, y cuando termines, vas a devolverle un número entero (será el 0 al final). Eso es todo, es como cuando alguien te dice "voy a hacer un pastel, y cuando termine te doy un pedazo", todo lo que está entre los corchetes es lo que le vas a meter al pastel, empezar de esa forma es para que el encargado sepa, "ah, esta es una receta formal, tiene un comienzo y un fin definido". Si empezaras directo con `decir("hola, mundo")`, el encargado se quedaría diciendo "¿y el final de la receta? ¿dónde vergas está el resultado?". Al poner primero el nombre y el tipo de retorno, estás trazando los límites para lo que quieras hacer, ya sabes cuándo empieza y cuándo termina la *funcionalidad*, por eso se llama así.

**Después va la acción en sí.** `decir("hola, mundo")`, esto es el corazón del programa, un lenguaje no es más que un arsenal de funciones que puedes utilizar y moldear a tu manera para hacer lo que desees en la computadora, va dentro de los corchetes, eso es el contenido de la función.  

**La confirmación justo antes del final.** `retornar 0` es la forma de decirle al sistema operativo (y a cualquier otro programa que pueda correr el tuyo después): "oye, yo terminé, y todo salió bien. El número 0 significa 'no hubo problemas'". Si lo pusieras al principio, como `retornar 0; decir("hola, mundo")`, el programa terminaría antes de mostrar nada, y verías nada en pantalla, o verías el texto pero el sistema pensaría que falló porque no esperaste el resultado final en el lugar correcto. Ponerlo al final es como decir "fin de la clase, nota 10": es el cierre definitivo. El encargado lee eso y sabe: "ah, la receta terminó, el resultado es 0, puedo pasar a lo siguiente o cerrar la ventana con la tranquilidad de que no pasó nada malo".

**Y el cierre.** El `}` al final de todo, considera que todo el contenido de la función estará entre estos corchetes, por tanto el principio es `{` y el final es `}`. { Es como un rollo, todo lo de en medio es el contenido a leer } <==(Justo así)

---

## Ejecuta TÚ programa

Abre la terminal. Esa ventana con letras negras que a veces aparece al iniciar la computadora, o que buscas en el menú de aplicaciones (a veces llamada "símbolo del sistema", "terminal" o "prompt de comando").

En esa ventana, escribe exactamente esto:

```
falcato corre hola.fc
```

Presiona Enter.

Deberías ver aparecer:

```
hola, mundo
```

**¡Eso es todo!** Acabas de escribir un programa, hacer que el compilador lo traduzca a lenguaje de máquina y ejecutarlo. Los tres pasos en uno. No necesitas saber de procesadores, sistemas operativos ni nada de eso. Escribiste algo que tiene sentido, le dijiste al encargado del almacén qué hacer, y él te respondió.

¿Por qué el 0? Porque le estás diciendo al sistema operativo: "yo fui, hice su trabajo y todo salió bien". El 0 es el lenguaje universal de "no hubo problemas". Si pusieras otro número, estarías diciendo "hubo un problema específico", pero por ahora, el 0 basta.

## ¿No funciona?

- **"falcato no se reconoce"** → revisa el archivo INSTALL.md para instalarla bien. Significa que la computadora no encuentra el idioma con el que hablarás.
- **Error de tipos** → significa que escribiste algo que no cuadra, como poner un número donde espera un texto. El compiler te dirá en qué línea y columna, y con un mensaje en español te dirá qué pasó. Es como si el encargado te dijera: "oye, ese casillero no sirve para guardar números, guarda texto ahí".
- **Olvidaste el punto y coma** → Falcato es preciso con los detalles, un `;` de más o de menos puede detener todo. Revisa la línea indicada, quítalo y listo.

---

← [01: Que es Falcato?](01-que-es-falcato.md) | [Indice](../GUIA.md) | [Siguiente: Variables →](03-variables.md)