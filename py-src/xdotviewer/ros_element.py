from dataclasses import dataclass
from enum import Enum


class ChartValue(Enum):
    CALLBACK_DURATION = "callback-duration"
    MESSAGE_LATENCY = "message-latency"
    ACTIVATIONS_DELAY = "activation-delay"
    MESSAGES_DELAY = "message-delay"
    PUBLICATIONS_DELAY = "publication-delay"

    @staticmethod
    def default_for(
        element_type: "ElementType", node_type: "NodeType | None"
    ) -> "ChartValue | None":
        if element_type == ElementType.EDGE:
            return ChartValue.MESSAGE_LATENCY
        match node_type:
            case NodeType.CALLBACK:
                return ChartValue.CALLBACK_DURATION
            case NodeType.TIMER:
                return ChartValue.ACTIVATIONS_DELAY
            case NodeType.PUBLISHER:
                return ChartValue.PUBLICATIONS_DELAY
            case NodeType.SUBSCRIBER:
                return ChartValue.MESSAGES_DELAY
            case NodeType.SERVICE:
                return None
        return None


class ChartType(Enum):
    HISTOGRAM = "histogram"
    SCATTER = "scatter"


class ElementType(Enum):
    NODE = "node"
    EDGE = "edge"


class NodeType(Enum):
    CALLBACK = "Callback"
    TIMER = "Timer"
    SERVICE = "Service"
    PUBLISHER = "Publisher"
    SUBSCRIBER = "Subscriber"


@dataclass
class ChartRequest:
    node: str
    value: ChartValue
    plot: ChartType
    bins: int | None = None
    size: tuple[int, int] = (800, 800)


@dataclass
class ElementReference:
    node: str
    element_type: ElementType
    node_type: NodeType | None = None

    @staticmethod
    def from_ref(ref: str) -> "ElementReference | None":
        if ref.startswith("r2ta-node://"):
            node, node_type = ref.removeprefix("r2ta-node://").split("|", 1)
            return ElementReference(
                node=node, element_type=ElementType.NODE, node_type=NodeType(node_type)
            )
        if ref.startswith("r2ta-edge://"):
            return ElementReference(
                node=ref.removeprefix("r2ta-edge://"),
                element_type=ElementType.EDGE,
                node_type=None,
            )
        return None
