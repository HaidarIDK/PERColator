"use strict";
/**
 * Type definitions for PERColator protocol
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.RouterInstruction = exports.SlabInstruction = exports.OrderState = exports.MakerClass = exports.TimeInForce = exports.Side = void 0;
/**
 * Trading side
 */
var Side;
(function (Side) {
    Side[Side["Buy"] = 0] = "Buy";
    Side[Side["Sell"] = 1] = "Sell";
})(Side || (exports.Side = Side = {}));
/**
 * Order time-in-force
 */
var TimeInForce;
(function (TimeInForce) {
    TimeInForce[TimeInForce["GTC"] = 0] = "GTC";
    TimeInForce[TimeInForce["IOC"] = 1] = "IOC";
    TimeInForce[TimeInForce["FOK"] = 2] = "FOK";
})(TimeInForce || (exports.TimeInForce = TimeInForce = {}));
/**
 * Maker class (DLP = Designated Liquidity Provider)
 */
var MakerClass;
(function (MakerClass) {
    MakerClass[MakerClass["Regular"] = 0] = "Regular";
    MakerClass[MakerClass["DLP"] = 1] = "DLP";
})(MakerClass || (exports.MakerClass = MakerClass = {}));
/**
 * Order state
 */
var OrderState;
(function (OrderState) {
    OrderState[OrderState["Live"] = 0] = "Live";
    OrderState[OrderState["Pending"] = 1] = "Pending";
})(OrderState || (exports.OrderState = OrderState = {}));
/**
 * Instruction discriminators
 */
var SlabInstruction;
(function (SlabInstruction) {
    SlabInstruction[SlabInstruction["Reserve"] = 0] = "Reserve";
    SlabInstruction[SlabInstruction["Commit"] = 1] = "Commit";
    SlabInstruction[SlabInstruction["Cancel"] = 2] = "Cancel";
    SlabInstruction[SlabInstruction["BatchOpen"] = 3] = "BatchOpen";
    SlabInstruction[SlabInstruction["Initialize"] = 4] = "Initialize";
    SlabInstruction[SlabInstruction["AddInstrument"] = 5] = "AddInstrument";
    SlabInstruction[SlabInstruction["UpdateFunding"] = 6] = "UpdateFunding";
    SlabInstruction[SlabInstruction["Liquidate"] = 7] = "Liquidate";
})(SlabInstruction || (exports.SlabInstruction = SlabInstruction = {}));
var RouterInstruction;
(function (RouterInstruction) {
    RouterInstruction[RouterInstruction["Initialize"] = 0] = "Initialize";
    RouterInstruction[RouterInstruction["Deposit"] = 1] = "Deposit";
    RouterInstruction[RouterInstruction["Withdraw"] = 2] = "Withdraw";
    RouterInstruction[RouterInstruction["MultiReserve"] = 3] = "MultiReserve";
    RouterInstruction[RouterInstruction["MultiCommit"] = 4] = "MultiCommit";
    RouterInstruction[RouterInstruction["Liquidate"] = 5] = "Liquidate";
})(RouterInstruction || (exports.RouterInstruction = RouterInstruction = {}));
