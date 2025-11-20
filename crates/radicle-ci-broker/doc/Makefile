.SUFFIXES: .uml .svg .dot .pik .md .html

.md.html:
	pandoc -V date="Version: $$(git describe --long --dirty --all)" --toc --number-sections --standalone --self-contained $< -o $@

.dot.svg:
	dot -Tsvg $< > $@.tmp
	mv $@.tmp $@

.uml.svg:
	plantuml -tsvg --output=. $<

.pik.svg:
	pikchr-cli $< > $@.tmp
	mv $@.tmp $@

all: architecture.html userguide.html

publish: all
	bash publish.sh

architecture.html: architecture.svg architecture-ext.svg comp.svg comp-ext.svg Makefile

userguide.html: userguide.subplot userguide.md Makefile
	subplot docgen $< --output $@
