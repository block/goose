# pylint: disable=useless-return, bad-staticmethod-argument, duplicate-code, import-outside-toplevel, broad-exception-caught, unused-argument, import-error, too-many-return-statements, superfluous-parens
"""Initializer of Auto Instrumentation of GPU Metrics"""

from typing import Collection, Iterable
import logging
from functools import partial
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from opentelemetry.sdk.resources import (
    SERVICE_NAME,
    TELEMETRY_SDK_NAME,
    DEPLOYMENT_ENVIRONMENT,
)
from opentelemetry.metrics import get_meter, CallbackOptions, Observation
from openlit.semcov import SemanticConvention

# Initialize logger for logging potential issues and operations
logger = logging.getLogger(__name__)


class GPUInstrumentor(BaseInstrumentor):
    """
    An instrumentor for collecting GPU metrics.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return []

    def _instrument(self, **kwargs):
        application_name = kwargs.get("application_name", "default")
        environment = kwargs.get("environment", "default")
        # pylint: disable=attribute-defined-outside-init
        self.gpu_type = self._get_gpu_type()
        meter = get_meter(
            __name__,
            "0.1.0",
            schema_url="https://opentelemetry.io/schemas/1.11.0",
        )

        if not self.gpu_type:
            logger.error(
                "OpenLIT GPU Instrumentation Error: No supported GPUs found."
                "If this is a non-GPU host, set `collect_gpu_stats=False` to disable GPU stats."
            )
            return

        metric_names = [
            ("GPU_UTILIZATION", "utilization"),
            ("GPU_UTILIZATION_ENC", "utilization_enc"),
            ("GPU_UTILIZATION_DEC", "utilization_dec"),
            ("GPU_TEMPERATURE", "temperature"),
            ("GPU_FAN_SPEED", "fan_speed"),
            ("GPU_MEMORY_AVAILABLE", "memory_available"),
            ("GPU_MEMORY_TOTAL", "memory_total"),
            ("GPU_MEMORY_USED", "memory_used"),
            ("GPU_MEMORY_FREE", "memory_free"),
            ("GPU_POWER_DRAW", "power_draw"),
            ("GPU_POWER_LIMIT", "power_limit"),
        ]

        for semantic_name, internal_name in metric_names:
            meter.create_observable_gauge(
                name=getattr(SemanticConvention, semantic_name),
                callbacks=[
                    partial(
                        self._collect_metric,
                        environment,
                        application_name,
                        internal_name,
                    )
                ],
                description=f"GPU {internal_name.replace('_', ' ').title()}",
            )

    def _uninstrument(self, **kwargs):
        # Proper uninstrumentation logic to revert patched methods
        pass

    def _get_gpu_type(self) -> str:
        try:
            import pynvml

            pynvml.nvmlInit()
            return "nvidia"
        except Exception:
            try:
                import amdsmi

                amdsmi.amdsmi_init()
                return "amd"
            except Exception:
                return None

    def _collect_metric(
        self, environment, application_name, metric_name, options: CallbackOptions
    ) -> Iterable[Observation]:
        # pylint: disable=no-else-return
        if self.gpu_type == "nvidia":
            return self._collect_nvidia_metrics(
                environment, application_name, metric_name, options
            )
        elif self.gpu_type == "amd":
            return self._collect_amd_metrics(
                environment, application_name, metric_name, options
            )
        return []

    def _collect_nvidia_metrics(
        self, environment, application_name, metric_name, options: CallbackOptions
    ) -> Iterable[Observation]:
        try:
            import pynvml

            gpu_count = pynvml.nvmlDeviceGetCount()
            mega_bytes = 1024 * 1024
            gpu_index = 0
            for gpu_index in range(gpu_count):
                handle = pynvml.nvmlDeviceGetHandleByIndex(gpu_index)

                def get_metric_value(handle, metric_name):
                    try:
                        # pylint: disable=no-else-return
                        if metric_name == "temperature":
                            return pynvml.nvmlDeviceGetTemperature(
                                handle, pynvml.NVML_TEMPERATURE_GPU
                            )
                        elif metric_name == "utilization":
                            return pynvml.nvmlDeviceGetUtilizationRates(handle).gpu
                        elif metric_name == "utilization_enc":
                            return pynvml.nvmlDeviceGetEncoderUtilization(handle)[0]
                        elif metric_name == "utilization_dec":
                            return pynvml.nvmlDeviceGetDecoderUtilization(handle)[0]
                        elif metric_name == "fan_speed":
                            return 0
                        elif metric_name == "memory_available":
                            memory_info = pynvml.nvmlDeviceGetMemoryInfo(handle)
                            return (
                                memory_info.free // mega_bytes
                            )  # Assuming reserved memory is 0
                        elif metric_name == "memory_total":
                            return (
                                pynvml.nvmlDeviceGetMemoryInfo(handle).total
                                // mega_bytes
                            )
                        elif metric_name == "memory_used":
                            return (
                                pynvml.nvmlDeviceGetMemoryInfo(handle).used
                                // mega_bytes
                            )
                        elif metric_name == "memory_free":
                            return (
                                pynvml.nvmlDeviceGetMemoryInfo(handle).free
                                // mega_bytes
                            )
                        elif metric_name == "power_draw":
                            return pynvml.nvmlDeviceGetPowerUsage(handle) // 1000.0
                        elif metric_name == "power_limit":
                            return (
                                pynvml.nvmlDeviceGetEnforcedPowerLimit(handle) // 1000.0
                            )
                    except Exception as e:
                        # pylint: disable=cell-var-from-loop
                        logger.error(
                            "Error collecting metric %s for GPU %d: %s",
                            metric_name,
                            gpu_index,
                            e,
                        )
                    return 0

                def safe_decode(byte_string):
                    if isinstance(byte_string, bytes):
                        return byte_string.decode("utf-8")
                    return byte_string

                attributes = {
                    TELEMETRY_SDK_NAME: "openlit",
                    SERVICE_NAME: application_name,
                    DEPLOYMENT_ENVIRONMENT: environment,
                    SemanticConvention.GPU_INDEX: str(gpu_index),
                    SemanticConvention.GPU_UUID: safe_decode(
                        pynvml.nvmlDeviceGetUUID(handle)
                    ),
                    SemanticConvention.GPU_NAME: safe_decode(
                        pynvml.nvmlDeviceGetName(handle)
                    ),
                }
                yield Observation(get_metric_value(handle, metric_name), attributes)

        except Exception as e:
            logger.error("Error in GPU metrics collection: %s", e)

    def _collect_amd_metrics(
        self, environment, application_name, metric_name, options: CallbackOptions
    ) -> Iterable[Observation]:
        try:
            import amdsmi

            # Get the number of AMD GPUs
            devices = amdsmi.amdsmi_get_processor_handles()
            mega_bytes = 1024 * 1024
            for device_handle in devices:

                def get_metric_value(device_handle, metric_name):
                    try:
                        # pylint: disable=no-else-return
                        if metric_name == "temperature":
                            # pylint: disable=line-too-long
                            return amdsmi.amdsmi_get_temp_metric(
                                device_handle,
                                amdsmi.AmdSmiTemperatureType.EDGE,
                                amdsmi.AmdSmiTemperatureMetric.CURRENT,
                            )
                        elif metric_name == "utilization":
                            # pylint: disable=line-too-long
                            return amdsmi.amdsmi_get_utilization_count(
                                device_handle,
                                amdsmi.AmdSmiUtilizationCounterType.COARSE_GRAIN_GFX_ACTIVITY,
                            )
                        elif metric_name in ["utilization_enc", "utilization_dec"]:
                            return 0  # Placeholder if unsupported
                        elif metric_name == "fan_speed":
                            return amdsmi.amdsmi_get_gpu_fan_speed(device_handle, 0)
                        elif metric_name == "memory_available":
                            return (
                                amdsmi.amdsmi_get_gpu_memory_total(device_handle)
                                // mega_bytes
                            )
                        elif metric_name == "memory_total":
                            return (
                                amdsmi.amdsmi_get_gpu_memory_total(device_handle)
                                // mega_bytes
                            )
                        elif metric_name == "memory_used":
                            return (
                                amdsmi.amdsmi_get_gpu_memory_usage(device_handle)
                                // mega_bytes
                            )
                        elif metric_name == "memory_free":
                            total_mem = (
                                amdsmi.amdsmi_get_gpu_memory_total(device_handle)
                                // mega_bytes
                            )
                            used_mem = (
                                amdsmi.amdsmi_get_gpu_memory_usage(device_handle)
                                // mega_bytes
                            )
                            return total_mem - used_mem
                        elif metric_name == "power_draw":
                            # pylint: disable=line-too-long
                            return (
                                amdsmi.amdsmi_get_power_info(device_handle)[
                                    "average_socket_power"
                                ]
                                // 1000.0
                            )
                        elif metric_name == "power_limit":
                            # pylint: disable=line-too-long
                            return (
                                amdsmi.amdsmi_get_power_info(device_handle)[
                                    "power_limit"
                                ]
                                // 1000.0
                            )
                    except Exception as e:
                        logger.error(
                            "Error collecting metric %s for AMD GPU %d: %s",
                            metric_name,
                            amdsmi.amdsmi_get_xgmi_info(device_handle)["index"],
                            e,
                        )
                    return 0

                attributes = {
                    TELEMETRY_SDK_NAME: "openlit",
                    SERVICE_NAME: application_name,
                    DEPLOYMENT_ENVIRONMENT: environment,
                    # pylint: disable=line-too-long
                    SemanticConvention.GPU_INDEX: amdsmi.amdsmi_get_xgmi_info(
                        device_handle
                    )["index"],
                    # pylint: disable=line-too-long
                    SemanticConvention.GPU_UUID: amdsmi.amdsmi_get_gpu_asic_info(
                        device_handle
                    )["market_name"],
                    SemanticConvention.GPU_NAME: amdsmi.amdsmi_get_device_name(
                        device_handle
                    ),
                }
                yield Observation(
                    get_metric_value(device_handle, metric_name), attributes
                )

        except Exception as e:
            logger.error("Error in AMD GPU metrics collection: %s", e)
