Config_Static = .config_static
Config_Dynamic = .config_dynamic

menuconfig-static:
	@export KCONFIG_CONFIG=$(Config_Static) && \
	export MENUCONFIG_STYLE=monochrome \
	&& menuconfig Kconfig_static

menuconfig-dynamic:
	@export KCONFIG_CONFIG=$(Config_Dynamic) && \
	export MENUCONFIG_STYLE=monochrome \
	&& menuconfig Kconfig_dynamic

clean: 
	@rm -f $(Config_Dynamic)
	@rm -f $(Config_Dynamic).old
	@rm -f $(Config_Static)
	@rm -f $(Config_Static).old

.PHONY: menuconfig-static menuconfig-dynamic clean