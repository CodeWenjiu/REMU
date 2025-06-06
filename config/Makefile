Config_Dynamic = .config_dynamic

menuconfig-static:
	@export KCONFIG_CONFIG=$(Config_Dynamic) && \
	export MENUCONFIG_STYLE=monochrome \
	&& menuconfig Kconfig_dynamic

clean: 
	@rm -f $(Config_Dynamic)
	@rm -f $(Config_Dynamic).old

.PHONY: menuconfig-static clean