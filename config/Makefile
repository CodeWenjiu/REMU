Static_Path = ./static
Dynamic_Path = ./dynamic

Config_Static = $(Static_Path)/.config
Config_Dynamic = $(Dynamic_Path)/.config

ALL_Static_CONFIG = $(shell find $(Static_Path) -name Kconfig)
ALL_Dynamic_CONFIG = $(shell find $(Dynamic_Path) -name Kconfig)

menuconfig-static:
	@export KCONFIG_CONFIG=$(Config_Static) && \
	export MENUCONFIG_STYLE=monochrome \
	&& menuconfig $(Static_Path)/Kconfig

menuconfig-dynamic: 
	@export KCONFIG_CONFIG=$(Config_Dynamic) && \
	export MENUCONFIG_STYLE=monochrome \
	&& menuconfig $(Dynamic_Path)/Kconfig

menuconfig: menuconfig-static menuconfig-dynamic

$(Config_Static): $(ALL_Static_CONFIG)
	@$(MAKE) menuconfig-static

$(Config_Dynamic): $(ALL_Dynamic_CONFIG)
	@$(MAKE) menuconfig-dynamic

menuconfig-static-conditional: $(Config_Static)

menuconfig-dynamic-conditional: $(Config_Dynamic)

clean: 
	@rm -f $(Config_Dynamic)
	@rm -f $(Config_Dynamic).old
	@rm -f $(Config_Static)
	@rm -f $(Config_Static).old

.PHONY: menuconfig menuconfig-static menuconfig-dynamic menuconfig-static-conditional menuconfig-dynamic-conditional clean 