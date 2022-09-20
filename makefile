TARGET := a.i
SOURCE := test/testest/d.txt
DEPS := $(SOURCE:%=%.d)
SDIR := test
OPT := -I $(SDIR) -MF $(DEPS)

$(TARGET): $(SOURCE)
	./target/debug/preprocess.exe $(OPT) $< -o $@

-include $(DEPS)
