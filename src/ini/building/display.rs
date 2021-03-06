use std::fmt::{Formatter, Error, Display};
use std::io::{Write};

use super::{BuildingType,
            BuildingSubtype,
            ResourceVisualization,
            Token,
           };


type IOResult = Result<(), std::io::Error>;

impl Token<'_> {
    pub fn serialize_token<W: Write>(&self, mut wr: W) -> IOResult {

        // float serialization precision
        const PREC: usize = 4;

        macro_rules! write_pts {
            ($pfx:expr, $($i:ident),+) => {{
                write!(wr, "{}", $pfx)?;
                $(write!(wr, "\r\n{:.prec$} {:.prec$} {:.prec$}", $i.x, $i.y, $i.z, prec = PREC)?;)+
                Ok(())
            }};
        }

        macro_rules! write_x_pts {
            ($pfx:expr, $x:expr, $($i:ident),+) => {{
                write!(wr, "{} {}", $pfx, $x)?;
                $(write!(wr, "\r\n{:.prec$} {:.prec$} {:.prec$}", $i.x, $i.y, $i.z, prec = PREC)?;)+
                Ok(())
            }};
        }

        macro_rules! write_tag_pts {
            ($pfx:expr, $tag:expr, $($i:ident),+) => {{
                write!(wr, "{}{}", $pfx, $tag)?;
                $(write!(wr, "\r\n{:.prec$} {:.prec$} {:.prec$}", $i.x, $i.y, $i.z, prec = PREC)?;)+
                Ok(())
            }};
        }

        match self {
            Self::VehicleStation((a, b))           => write_pts!(Self::VEHICLE_STATION, a, b),
            Self::VehicleStationDetourPoint(p)     => write_pts!(Self::VEHICLE_STATION_DETOUR_POINT, p),
            Self::VehicleStationDetourPid((i, p))  => write_x_pts!(Self::VEHICLE_STATION_DETOUR_PID, i, p),

            Self::VehicleParking((a, b))           => write_pts!(Self::VEHICLE_PARKING, a, b),
            Self::VehicleParkingDetourPoint(p)     => write_pts!(Self::VEHICLE_PARKING_DETOUR_POINT, p),
            Self::VehicleParkingDetourPid((i, p))  => write_x_pts!(Self::VEHICLE_PARKING_DETOUR_PID, i, p),
            Self::VehicleParkingPersonal((a, b))   => write_pts!(Self::VEHICLE_PARKING_PERSONAL, a, b),

            Self::AirplaneStation((t, p1, p2))     => write_tag_pts!(Self::AIRPLANE_STATION, t, p1, p2),
            Self::HeliportStation((a, b))          => write_pts!(Self::HELIPORT_STATION, a, b),
            Self::ShipStation((a, b))              => write_pts!(Self::SHIP_STATION, a, b),

            Self::Connection2Points((t, p1, p2))   => write_tag_pts!(Self::CONNECTION, t, p1, p2),
            Self::Connection1Point((t, p))         => write_tag_pts!(Self::CONNECTION, t, p),
            Self::OffsetConnection((i, p))         => write_x_pts!(Self::OFFSET_CONNECTION_XYZW, i, p),

            Self::ConnectionsSpace(r)                => write!(wr, "{}\r\n{:.prec$} {:.prec$}\r\n{:.prec$} {:.prec$}",         
                                                               Self::CONNECTIONS_SPACE, r.x1, r.z1, r.x2, r.z2, prec = PREC),
            Self::ConnectionsRoadDeadSquare(r)       => write!(wr, "{}\r\n{:.prec$} {:.prec$}\r\n{:.prec$} {:.prec$}",
                                                               Self::CONNECTIONS_ROAD_DEAD_SQUARE,    r.x1, r.z1, r.x2, r.z2, prec = PREC),
            Self::ConnectionsAirportDeadSquare(r)    => write!(wr, "{}\r\n{:.prec$} {:.prec$}\r\n{:.prec$} {:.prec$}",
                                                               Self::CONNECTIONS_AIRPORT_DEAD_SQUARE, r.x1, r.z1, r.x2, r.z2, prec = PREC),
            Self::ConnectionsWaterDeadSquare((x, r)) => write!(wr, "{}\r\n{:.prec$}\r\n{:.prec$} {:.prec$}\r\n{:.prec$} {:.prec$}",
                                                               Self::CONNECTIONS_ROAD_DEAD_SQUARE, x, r.x1, r.z1, r.x2, r.z2, prec = PREC),

            Self::Particle((t, p, a, s))           => write!(wr, "{} {} {:.prec$} {:.prec$} {:.prec$} {:.prec$} {:.prec$}", 
                                                             Self::PARTICLE, t, p.x, p.y, p.z, a, s, prec = PREC),
            Self::ParticleReactor(p)               => write_pts!(Self::PARTICLE_REACTOR, p),
            Self::ParticleSnowRemove((p, i, r))    => write!(wr, "{} {:.prec$} {:.prec$} {:.prec$} {} {:.prec$}", 
                                                             Self::PARTICLE_SNOW_REMOVE, p.x, p.y, p.z, i, r, prec = PREC),
            Self::TextCaption((a, b))              => write_pts!(Self::TEXT_CAPTION, a, b),
            Self::WorkerRenderingArea((a, b))      => write_pts!(Self::WORKER_RENDERING_AREA, a, b),
            Self::ResourceVisualization(ResourceVisualization { storage_id, position: p, rotation, scale: s, numstep_x: (x1, x2), numstep_z: (z1, z2) }) => 
                write!(wr, "{} {}\nposition {:.prec$} {:.prec$} {:.prec$}\n\
                                  rotation {:.prec$}\n\
                                  scale {:.prec$} {:.prec$} {:.prec$}\n\
                                  numstep_x {:.prec$} {}\n\
                                  numstep_t {:.prec$} {}", 
                       Self::RESOURCE_VISUALIZATION, storage_id, p.x, p.y, p.z, rotation, s.x, s.y, s.z, x1, x2, z1, z2, prec = PREC),
            Self::ResourceIncreasePoint((i, p))        => write_x_pts!(Self::RESOURCE_INCREASE_POINT, i, p),
            Self::ResourceIncreaseConvPoint((i, a, b)) => write_x_pts!(Self::RESOURCE_INCREASE_CONV_POINT, i, a, b),
            Self::ResourceFillingPoint(p)              => write_pts!(Self::RESOURCE_FILLING_POINT, p),
            Self::ResourceFillingConvPoint((a, b))     => write_pts!(Self::RESOURCE_FILLING_CONV_POINT, a, b),

            Self::CostWorkVehicleStation((a, b))       => write_pts!(Self::COST_WORK_VEHICLE_STATION, a, b),

            t => write!(wr, "{}", t)
        }

    }
}

impl Display for Token<'_> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::NameStr(p)                       => write!(f, "{} {}", Self::NAME_STR, p),
            Self::Name(p)                          => write!(f, "{} {}", Self::NAME, p),
            Self::BuildingType(p)                  => write!(f, "{}{}",  Self::BUILDING_TYPE, p),
            Self::BuildingSubtype(p)               => write!(f, "{}{}",  Self::BUILDING_SUBTYPE, p),

            Self::HeatEnable                       => write!(f, "{}",    Self::HEATING_ENABLE),
            Self::HeatDisable                      => write!(f, "{}",    Self::HEATING_DISABLE),
            Self::CivilBuilding                    => write!(f, "{}",    Self::CIVIL_BUILDING),
            Self::MonumentTrespass                 => write!(f, "{}",    Self::MONUMENT_TRESPASS),
            Self::QualityOfLiving(x)               => write!(f, "{} {}", Self::QUALITY_OF_LIVING, x),

            Self::WorkersNeeded(x)                 => write!(f, "{} {}",    Self::WORKERS_NEEDED, x),
            Self::ProfessorsNeeded(x)              => write!(f, "{} {}",    Self::PROFESSORS_NEEDED, x),
            Self::CitizenAbleServe(x)              => write!(f, "{} {}",    Self::CITIZEN_ABLE_SERVE, x),
            Self::Consumption((t, x))              => write!(f, "{} {} {}", Self::CONSUMPTION, t, x),
            Self::ConsumptionPerSec((t, x))        => write!(f, "{} {} {}", Self::CONSUMPTION_PER_SEC, t, x),
            Self::Production((t, x))               => write!(f, "{} {} {}", Self::PRODUCTION, t, x),
            Self::ProductionSun(x)                 => write!(f, "{} {}",    Self::PRODUCTION_SUN, x),
            Self::ProductionWind(x)                => write!(f, "{} {}",    Self::PRODUCTION_WIND, x),
            Self::SeasonalTempMin(x)               => write!(f, "{} {}",    Self::SEASONAL_TEMP_MIN, x),
            Self::SeasonalTempMax(x)               => write!(f, "{} {}",    Self::SEASONAL_TEMP_MAX, x),

            Self::EleConsumWorkerFactorBase(x)     => write!(f, "{} {}", Self::ELE_CONSUM_WORKER_FACTOR_BASE, x),
            Self::EleConsumWorkerFactorNight(x)    => write!(f, "{} {}", Self::ELE_CONSUM_WORKER_FACTOR_NIGHT, x),
            Self::EleConsumServeFactorBase(x)      => write!(f, "{} {}", Self::ELE_CONSUM_SERVE_FACTOR_BASE, x),
            Self::EleConsumServeFactorNight(x)     => write!(f, "{} {}", Self::ELE_CONSUM_SERVE_FACTOR_NIGHT, x),
            Self::EleConsumCargoLoadFactor(x)      => write!(f, "{} {}", Self::ELE_CONSUM_CARGO_LOAD_FACTOR, x),
            Self::EleConsumCargoUnloadFactor(x)    => write!(f, "{} {}", Self::ELE_CONSUM_CARGO_UNLOAD_FACTOR, x),

            Self::NoEleWorkFactorBase(x)           => write!(f, "{} {}", Self::NO_ELE_WORK_FACTOR_BASE, x),
            Self::NoEleWorkFactorNight(x)          => write!(f, "{} {}", Self::NO_ELE_WORK_FACTOR_NIGHT, x),
            Self::NoHeatWorkFactor(x)              => write!(f, "{} {}", Self::NO_HEAT_WORK_FACTOR, x),

            Self::EngineSpeed(x)                   => write!(f, "{} {}", Self::ENGINE_SPEED, x),
            Self::CablewayHeavy                    => write!(f, "{}",    Self::CABLEWAY_HEAVY),
            Self::CablewayLight                    => write!(f, "{}",    Self::CABLEWAY_LIGHT),
            Self::ResourceSource(t)                => write!(f, "{}{}",  Self::RESOURCE_SOURCE, t),

            Self::Storage((t, x))                  => write!(f, "{} {} {}",    Self::STORAGE, t, x),
            Self::StorageSpecial((t, x, r))        => write!(f, "{} {} {} {}", Self::STORAGE_SPECIAL, t, x, r),
            Self::StorageFuel((t, x))              => write!(f, "{} {} {}",    Self::STORAGE_FUEL, t, x),
            Self::StorageExport((t, x))            => write!(f, "{} {} {}",    Self::STORAGE_EXPORT, t, x),
            Self::StorageImport((t, x))            => write!(f, "{} {} {}",    Self::STORAGE_IMPORT, t, x),
            Self::StorageImportCarplant((t, x))    => write!(f, "{} {} {}",    Self::STORAGE_IMPORT_CARPLANT, t, x),
            Self::StorageExportSpecial((t, x, r))  => write!(f, "{} {} {} {}", Self::STORAGE_EXPORT_SPECIAL, t, x, r),
            Self::StorageImportSpecial((t, x, r))  => write!(f, "{} {} {} {}", Self::STORAGE_IMPORT_SPECIAL, t, x, r),
            Self::StorageDemandBasic((t, x))       => write!(f, "{} {} {}",    Self::STORAGE_DEMAND_BASIC, t, x),
            Self::StorageDemandMediumAdvanced((t, x))
                                                   => write!(f, "{} {} {}",    Self::STORAGE_DEMAND_MEDIUMADVANCED, t, x),
            Self::StorageDemandAdvanced((t, x))    => write!(f, "{} {} {}",    Self::STORAGE_DEMAND_ADVANCED, t, x),
            Self::StorageDemandHotel((t, x))       => write!(f, "{} {} {}",    Self::STORAGE_DEMAND_HOTEL, t, x),
            Self::StoragePackFrom(i)               => write!(f, "{} {}",       Self::STORAGE_PACK_FROM, i),
            Self::StorageUnpackTo(i)               => write!(f, "{} {}",       Self::STORAGE_UNPACK_TO, i),
            Self::StorageLivingAuto(id)            => write!(f, "{} {}",       Self::STORAGE_LIVING_AUTO, id),

            Self::VehicleLoadingFactor(x)          => write!(f, "{} {}",    Self::VEHICLE_LOADING_FACTOR, x),
            Self::VehicleUnloadingFactor(x)        => write!(f, "{} {}",    Self::VEHICLE_UNLOADING_FACTOR, x),

            Self::RoadNotFlip                      => write!(f, "{}",       Self::ROAD_VEHICLE_NOT_FLIP),
            Self::RoadElectric                     => write!(f, "{}",       Self::ROAD_VEHICLE_ELECTRIC),
            Self::VehicleCannotSelect              => write!(f, "{}",       Self::VEHICLE_CANNOT_SELECT),
            Self::LongTrains                       => write!(f, "{}",       Self::LONG_TRAINS),

            Self::WorkingVehiclesNeeded(x)         => write!(f, "{} {}",    Self::WORKING_VEHICLES_NEEDED, x),
            Self::VehicleStation((a, b))           => write!(f, "{} {} {}", Self::VEHICLE_STATION, a, b),
            Self::VehicleStationNotBlock           => write!(f, "{}",       Self::VEHICLE_STATION_NOT_BLOCK),
            Self::VehicleStationDetourPoint(p)     => write!(f, "{} {}",    Self::VEHICLE_STATION_DETOUR_POINT, p),
            Self::VehicleStationDetourPid((i, p))  => write!(f, "{} {} {}", Self::VEHICLE_STATION_DETOUR_PID, i, p),

            Self::VehicleParking((a, b))           => write!(f, "{} {} {}", Self::VEHICLE_PARKING, a, b),
            Self::VehicleParkingDetourPoint(p)     => write!(f, "{} {}",    Self::VEHICLE_PARKING_DETOUR_POINT, p),
            Self::VehicleParkingDetourPid((i, p))  => write!(f, "{} {} {}", Self::VEHICLE_PARKING_DETOUR_PID, i, p),
            Self::VehicleParkingPersonal((a, b))   => write!(f, "{} {} {}", Self::VEHICLE_PARKING_PERSONAL, a, b),

            Self::AirplaneStation((t, a, b))       => write!(f, "{}{} {} {}", Self::AIRPLANE_STATION, t, a, b),
            Self::HeliportStation((a, b))          => write!(f, "{} {} {}",   Self::HELIPORT_STATION, a, b),
            Self::ShipStation((a, b))              => write!(f, "{} {} {}",   Self::SHIP_STATION, a, b),
            Self::HeliportArea(x)                  => write!(f, "{} {}",      Self::HELIPORT_AREA, x),
            Self::HarborTerrainFrom(x)             => write!(f, "{} {}",      Self::HARBOR_OVER_TERRAIN_FROM, x),
            Self::HarborWaterFrom(x)               => write!(f, "{} {}",      Self::HARBOR_OVER_WATER_FROM, x),
            Self::HarborExtendWhenBuilding(x)      => write!(f, "{} {}",      Self::HARBOR_EXTEND_WHEN_BULDING, x),
 
            Self::Connection2Points((t, a, b))     => write!(f, "{}{} {} {}", Self::CONNECTION, t, a, b),
            Self::Connection1Point((t, p))         => write!(f, "{}{} {}",    Self::CONNECTION, t, p),
            Self::OffsetConnection((i, a))         => write!(f, "{} {} {}",   Self::OFFSET_CONNECTION_XYZW, i, a),
            Self::ConnectionRailDeadend            => write!(f, "{}{}",       Self::CONNECTION, Self::CONNECTION_RAIL_DEADEND),

            Self::ConnectionsSpace(r)              => write!(f, "{} {}",      Self::CONNECTIONS_SPACE, r),
            Self::ConnectionsRoadDeadSquare(r)     => write!(f, "{} {}",      Self::CONNECTIONS_ROAD_DEAD_SQUARE, r),
            Self::ConnectionsAirportDeadSquare(r)  => write!(f, "{} {}",      Self::CONNECTIONS_AIRPORT_DEAD_SQUARE, r),
            Self::ConnectionsWaterDeadSquare((x, r)) => write!(f, "{} {} {}", Self::CONNECTIONS_WATER_DEAD_SQUARE, x, r),

            Self::AttractionType((t, x))           => write!(f, "{}{} {}", Self::ATTRACTION_TYPE, t, x),
            Self::AttractionRememberUsage          => write!(f, "{}",      Self::ATTRACTION_REMEMBER_USAGE),
            Self::AttractiveScoreBase(x)           => write!(f, "{} {}",   Self::ATTRACTIVE_SCORE_BASE, x),
            Self::AttractiveScoreAlcohol(x)        => write!(f, "{} {}",   Self::ATTRACTIVE_SCORE_ALCOHOL, x),
            Self::AttractiveScoreCulture(x)        => write!(f, "{} {}",   Self::ATTRACTIVE_SCORE_CULTURE, x),
            Self::AttractiveScoreReligion(x)       => write!(f, "{} {}",   Self::ATTRACTIVE_SCORE_RELIGION, x),
            Self::AttractiveScoreSport(x)          => write!(f, "{} {}",   Self::ATTRACTIVE_SCORE_SPORT, x),
            Self::AttractiveFactorNature(x)        => write!(f, "{} {}",   Self::ATTRACTIVE_FACTOR_NATURE, x),
            Self::AttractiveFactorNatureAdd(x)     => write!(f, "{} {}",   Self::ATTRACTIVE_FACTOR_NATURE_ADD, x),
            Self::AttractiveFactorPollution(x)     => write!(f, "{} {}",   Self::ATTRACTIVE_FACTOR_POLLUTION, x),
            Self::AttractiveFactorPollutionAdd(x)  => write!(f, "{} {}",   Self::ATTRACTIVE_FACTOR_POLLUTION_ADD, x),
            Self::AttractiveFactorSight(x)         => write!(f, "{} {}",   Self::ATTRACTIVE_FACTOR_SIGHT, x),
            Self::AttractiveFactorSightAdd(x)      => write!(f, "{} {}",   Self::ATTRACTIVE_FACTOR_SIGHT_ADD, x),
            Self::AttractiveFactorWater(x)         => write!(f, "{} {}",   Self::ATTRACTIVE_FACTOR_WATER, x),
            Self::AttractiveFactorWaterAdd(x)      => write!(f, "{} {}",   Self::ATTRACTIVE_FACTOR_WATER_ADD, x),

            Self::PollutionHigh                    => write!(f, "{}",      Self::POLLUTION_HIGH),
            Self::PollutionMedium                  => write!(f, "{}",      Self::POLLUTION_MEDIUM),
            Self::PollutionSmall                   => write!(f, "{}",      Self::POLLUTION_SMALL),

            Self::Particle((t, p, a, s))           => write!(f, "{} {} {} {} {}", Self::PARTICLE, t, p, a, s),
            Self::ParticleReactor(p)               => write!(f, "{} {}",       Self::PARTICLE_REACTOR, p),
            Self::ParticleSnowRemove((p, i, r))    => write!(f, "{} {} {} {}", Self::PARTICLE_SNOW_REMOVE, p, i, r),
            Self::TextCaption((a, b))              => write!(f, "{} {} {}",    Self::TEXT_CAPTION, a, b),
            Self::WorkerRenderingArea((a, b))      => write!(f, "{} {} {}",    Self::WORKER_RENDERING_AREA, a, b),
            Self::ResourceVisualization(rv)        => write!(f, "{} {}\nposition: {}\nrotation: {}\nscale: {}\nnumstep_x: {:?}\nnumstep_t: {:?}", 
                                                             Self::RESOURCE_VISUALIZATION, rv.storage_id, rv.position, rv.rotation, rv.scale, rv.numstep_x, rv.numstep_z),
            Self::ResourceIncreasePoint((i, a))    => write!(f, "{} {} {}",    Self::RESOURCE_INCREASE_POINT, i, a),
            Self::ResourceIncreaseConvPoint((i, a, b))
                                                   => write!(f, "{} {} {} {}", Self::RESOURCE_INCREASE_CONV_POINT, i, a, b),
            Self::ResourceFillingPoint(a)          => write!(f, "{} {}",       Self::RESOURCE_FILLING_POINT, a),
            Self::ResourceFillingConvPoint((a, b)) => write!(f, "{} {} {}",    Self::RESOURCE_FILLING_CONV_POINT, a, b),

            Self::WorkingSfx(s)                    => write!(f, "{} {}",       Self::WORKING_SFX, s),
            Self::AnimationFps(x)                  => write!(f, "{} {}",       Self::ANIMATION_FPS, x),
            Self::AnimationMesh((s, t))            => write!(f, "{} \"{}\" \"{}\"", Self::ANIMATION_MESH, s, t),
            Self::UndergroundMesh((s, t))          => write!(f, "{} \"{}\" \"{}\"", Self::UNDERGROUND_MESH, s, t),

            Self::CostWork((t, x))                 => write!(f, "{} {} {}", Self::COST_WORK, t, x),
            Self::CostWorkBuildingNode(n)          => write!(f, "{} {}",    Self::COST_WORK_BUILDING_NODE, n),
            Self::CostWorkBuildingKeyword(n)       => write!(f, "{} {}",    Self::COST_WORK_BUILDING_KEYWORD, n),
            Self::CostWorkBuildingAll              => write!(f, "{}",       Self::COST_WORK_BUILDING_ALL),

            Self::CostResource((t, x))             => write!(f, "{} {} {}", Self::COST_RESOURCE, t, x),
            Self::CostResourceAuto((t, x))         => write!(f, "{} {} {}", Self::COST_RESOURCE_AUTO, t, x),
            Self::CostWorkVehicleStation((a, b))   => write!(f, "{} {} {}", Self::COST_WORK_VEHICLE_STATION, a, b),
            Self::CostWorkVehicleStationNode(p)    => write!(f, "{} {}",    Self::COST_WORK_VEHICLE_STATION_NODE, p),
        }
    }
}


impl Display for BuildingType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = match self {
            Self::AirplaneGate           => Self::TYPE_AIRPLANE_GATE,
            Self::AirplaneParking        => Self::TYPE_AIRPLANE_PARKING,
            Self::AirplaneTower          => Self::TYPE_AIRPLANE_TOWER,
            Self::Attraction             => Self::TYPE_ATTRACTION,
            Self::Broadcast              => Self::TYPE_BROADCAST,
            Self::CarDealer              => Self::TYPE_CAR_DEALER,
            Self::CargoStation           => Self::TYPE_CARGO_STATION,
            Self::Church                 => Self::TYPE_CHURCH,
            Self::Cityhall               => Self::TYPE_CITYHALL,
            Self::ConstructionOffice     => Self::TYPE_CONSTRUCTION_OFFICE,
            Self::ConstructionOfficeRail => Self::TYPE_CONSTRUCTION_OFFICE_RAIL,
            Self::ContainerFacility      => Self::TYPE_CONTAINER_FACILITY,
            Self::CoolingTower           => Self::TYPE_COOLING_TOWER,
            Self::Customhouse            => Self::TYPE_CUSTOMHOUSE,
            Self::DistributionOffice     => Self::TYPE_DISTRIBUTION_OFFICE,
            Self::ElectricExport         => Self::TYPE_ELETRIC_EXPORT,
            Self::ElectricImport         => Self::TYPE_ELETRIC_IMPORT,
            Self::Engine                 => Self::TYPE_ENGINE,
            Self::Factory                => Self::TYPE_FACTORY,
            Self::Farm                   => Self::TYPE_FARM,
            Self::Field                  => Self::TYPE_FIELD,
            Self::Firestation            => Self::TYPE_FIRESTATION,
            Self::ForkliftGarage         => Self::TYPE_FORKLIFT_GARAGE,
            Self::GarbageOffice          => Self::TYPE_GARBAGE_OFFICE,
            Self::GasStation             => Self::TYPE_GAS_STATION,
            Self::HeatingEndstation      => Self::TYPE_HEATING_ENDSTATION,
            Self::HeatingPlant           => Self::TYPE_HEATING_PLANT,
            Self::HeatingSwitch          => Self::TYPE_HEATING_SWITCH,
            Self::Hospital               => Self::TYPE_HOSPITAL,
            Self::Hotel                  => Self::TYPE_HOTEL,
            Self::Kindergarten           => Self::TYPE_KINDERGARTEN,
            Self::Kino                   => Self::TYPE_KINO,
            Self::Living                 => Self::TYPE_LIVING,
            Self::MineBauxite            => Self::TYPE_MINE_BAUXITE,
            Self::MineCoal               => Self::TYPE_MINE_COAL,
            Self::MineGravel             => Self::TYPE_MINE_GRAVEL,
            Self::MineIron               => Self::TYPE_MINE_IRON,
            Self::MineOil                => Self::TYPE_MINE_OIL,
            Self::MineUranium            => Self::TYPE_MINE_URANIUM,
            Self::MineWood               => Self::TYPE_MINE_WOOD,
            Self::Monument               => Self::TYPE_MONUMENT,
            Self::Parking                => Self::TYPE_PARKING,
            Self::PassangerStation       => Self::TYPE_PASSANGER_STATION,
            Self::PedestrianBridge       => Self::TYPE_PEDESTRIAN_BRIDGE,
            Self::PoliceStation          => Self::TYPE_POLICE_STATION,
            Self::PollutionMeter         => Self::TYPE_POLLUTION_METER,
            Self::Powerplant             => Self::TYPE_POWERPLANT,
            Self::ProductionLine         => Self::TYPE_PRODUCTION_LINE,
            Self::Pub                    => Self::TYPE_PUB,
            Self::RailTrafo              => Self::TYPE_RAIL_TRAFO,
            Self::Raildepo               => Self::TYPE_RAILDEPO,
            Self::Roaddepo               => Self::TYPE_ROADDEPO,
            Self::School                 => Self::TYPE_SCHOOL,
            Self::ShipDock               => Self::TYPE_SHIP_DOCK,
            Self::Shop                   => Self::TYPE_SHOP,
            Self::Sport                  => Self::TYPE_SPORT,
            Self::Storage                => Self::TYPE_STORAGE,
            Self::Substation             => Self::TYPE_SUBSTATION,
            Self::Transformator          => Self::TYPE_TRANSFORMATOR,
            Self::University             => Self::TYPE_UNIVERSITY,
        };

        write!(f, "{}", s)
    }
}


impl Display for BuildingSubtype {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = match self {
            Self::Aircustom        => Self::SUBTYPE_AIRCUSTOM,
            Self::Airplane         => Self::SUBTYPE_AIRPLANE,
            Self::Cableway         => Self::SUBTYPE_CABLEWAY,
            Self::Hostel           => Self::SUBTYPE_HOSTEL,
            Self::Medical          => Self::SUBTYPE_MEDICAL,
            Self::Radio            => Self::SUBTYPE_RADIO,
            Self::Rail             => Self::SUBTYPE_RAIL,
            Self::Restaurant       => Self::SUBTYPE_RESTAURANT,
            Self::Road             => Self::SUBTYPE_ROAD,
            Self::Ship             => Self::SUBTYPE_SHIP,
            Self::Soviet           => Self::SUBTYPE_SOVIET,
            Self::SpaceForVehicles => Self::SUBTYPE_SPACE_FOR_VEHICLES,
            Self::Technical        => Self::SUBTYPE_TECHNICAL,
            Self::Television       => Self::SUBTYPE_TELEVISION,
            Self::Trolleybus       => Self::SUBTYPE_TROLLEYBUS,
        };

        write!(f, "{}", s)
    }
}


impl Display for super::Connection2PType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = match self {
            Self::AirRoad           => Self::CONN_AIRROAD,
            Self::Pedestrian        => Self::CONN_PED,
            Self::PedestrianNotPick => Self::CONN_PED_NOTPICK,
            Self::Road              => Self::CONN_ROAD,
            Self::RoadAllowpass     => Self::CONN_ROAD_ALLOWPASS,
            Self::RoadBorder        => Self::CONN_ROAD_BORDER,
            Self::RoadIn            => Self::CONN_ROAD_IN,
            Self::RoadOut           => Self::CONN_ROAD_OUT,
            Self::Rail              => Self::CONN_RAIL,
            Self::RailAllowpass     => Self::CONN_RAIL_ALLOWPASS,
            Self::RailBorder        => Self::CONN_RAIL_BORDER,
            Self::RailHeight        => Self::CONN_RAIL_HEIGHT,
            Self::HeatingBig        => Self::CONN_HEATING_BIG,
            Self::HeatingSmall      => Self::CONN_HEATING_SMALL,
            Self::SteamIn           => Self::CONN_STEAM_IN,
            Self::SteamOut          => Self::CONN_STEAM_OUT,
            Self::PipeIn            => Self::CONN_PIPE_IN,
            Self::PipeOut           => Self::CONN_PIPE_OUT,
            Self::BulkIn            => Self::CONN_BULK_IN,
            Self::BulkOut           => Self::CONN_BULK_OUT,
            Self::Cableway          => Self::CONN_CABLEWAY,
            Self::Factory           => Self::CONN_FACTORY,
            Self::ConveyorIn        => Self::CONN_CONVEYOR_IN,
            Self::ConveyorOut       => Self::CONN_CONVEYOR_OUT,
            Self::ElectricHighIn    => Self::CONN_ELECTRIC_H_IN,
            Self::ElectricHighOut   => Self::CONN_ELECTRIC_H_OUT,
            Self::ElectricLowIn     => Self::CONN_ELECTRIC_L_IN,
            Self::ElectricLowOut    => Self::CONN_ELECTRIC_L_OUT,
            Self::Fence             => Self::CONN_FENCE,
        };

        write!(f, "{}", s)
    }
}


impl Display for super::Connection1PType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = match self {
            Self::RoadDead       => Self::ROAD_DEAD,
            Self::PedestrianDead => Self::PEDESTRIAN_DEAD,
            Self::WaterDead      => Self::WATER_DEAD,
            Self::AirportDead    => Self::AIRPORT_DEAD,
            Self::AdvancedPoint  => Self::ADVANCED_POINT,
        };

        write!(f, "{}", s)
    }
}


impl Display for super::StorageCargoType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = match self {
            Self::Passanger => Self::PASSANGER,
            Self::Cement    => Self::CEMENT,
            Self::Covered   => Self::COVERED,
            Self::Gravel    => Self::GRAVEL,
            Self::Oil       => Self::OIL,
            Self::Open      => Self::OPEN,
            Self::Cooler    => Self::COOLER,
            Self::Concrete  => Self::CONCRETE,
            Self::Livestock => Self::LIVESTOCK,
            Self::General   => Self::GENERAL,
            Self::Vehicles  => Self::VEHICLES,
            Self::Nuclear1  => Self::NUCLEAR1,
            Self::Nuclear2  => Self::NUCLEAR2,
        };

        write!(f, "{}", s)
    }
}


impl Display for super::ConstructionAutoCost {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = match self {
            Self::Ground           => Self::GROUND,
            Self::GroundAsphalt    => Self::GROUND_ASPHALT,
            Self::WallConcrete     => Self::WALL_CONCRETE,
            Self::WallPanels       => Self::WALL_PANELS,
            Self::WallBrick        => Self::WALL_BRICK,
            Self::WallSteel        => Self::WALL_STEEL,
            Self::WallWood         => Self::WALL_WOOD,
            Self::TechSteel        => Self::TECH_STEEL,
            Self::ElectroSteel     => Self::ELECTRO_STEEL,
            Self::TechElectroSteel => Self::TECH_ELECTRO_STEEL,
            Self::RoofWoodBrick    => Self::ROOF_WOOD_BRICK,
            Self::RoofSteel        => Self::ROOF_STEEL,
            Self::RoofWoodSteel    => Self::ROOF_WOOD_STEEL,
        };

        write!(f, "{}", s)
    }
}


impl Display for super::ConstructionPhase {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = match self {
            Self::AsphaltLaying   => Self::ASPHALT_LAYING,
            Self::AsphaltRolling  => Self::ASPHALT_ROLLING,
            Self::BoardsLaying    => Self::BOARDS_LAYING,
            Self::BricksLaying    => Self::BRICKS_LAYING,
            Self::BridgeBuilding  => Self::BRIDGE_BUILDING,
            Self::GravelLaying    => Self::GRAVEL_LAYING,
            Self::Groundworks     => Self::GROUNDWORKS,
            Self::InteriorWorks   => Self::INTERIOR_WORKS,
            Self::PanelsLaying    => Self::PANELS_LAYING,
            Self::RailwayLaying   => Self::RAILWAY_LAYING,
            Self::RooftopBuilding => Self::ROOFTOP_BUILDING,
            Self::SkeletonCasting => Self::SKELETON_CASTING,
            Self::SteelLaying     => Self::STEEL_LAYING,
            Self::Tunneling       => Self::TUNNELING,
            Self::WireLaying      => Self::WIRE_LAYING,
        };

        write!(f, "{}", s)
    }
}


impl Display for super::ResourceType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = match self {
            Self::Alcohol           => Self::ALCOHOL,
            Self::Alumina           => Self::ALUMINA,
            Self::Aluminium         => Self::ALUMINIUM,
            Self::Asphalt           => Self::ASPHALT,
            Self::Bauxite           => Self::BAUXITE,
            Self::Bitumen           => Self::BITUMEN,
            Self::Boards            => Self::BOARDS,
            Self::Bricks            => Self::BRICKS,
            Self::Cement            => Self::CEMENT,
            Self::Chemicals         => Self::CHEMICALS,
            Self::Clothes           => Self::CLOTHES,
            Self::Coal              => Self::COAL,
            Self::Concrete          => Self::CONCRETE,
            Self::Crops             => Self::CROPS,
            Self::ElectroComponents => Self::ELECTRO_COMP,
            Self::Electricity       => Self::ELECTRICITY,
            Self::Electronics       => Self::ELECTRONICS,
            Self::Fabric            => Self::FABRIC,
            Self::Food              => Self::FOOD,
            Self::Fuel              => Self::FUEL,
            Self::Gravel            => Self::GRAVEL,
            Self::Heat              => Self::HEAT,
            Self::Iron              => Self::IRON,
            Self::Livestock         => Self::LIVESTOCK,
            Self::MechComponents    => Self::MECH_COMP,
            Self::Meat              => Self::MEAT,
            Self::NuclearFuel       => Self::NUCLEAR_FUEL,
            Self::NuclearWaste      => Self::NUCLEAR_WASTE,
            Self::Oil               => Self::OIL,
            Self::Plastic           => Self::PLASTIC,
            Self::PrefabPanels      => Self::PREFABS,
            Self::RawBauxite        => Self::RAW_BAUXITE,
            Self::RawCoal           => Self::RAW_COAL,
            Self::RawGravel         => Self::RAW_GRAVEL,
            Self::RawIron           => Self::RAW_IRON,
            Self::Steel             => Self::STEEL,
            Self::UF6               => Self::UF_6,
            Self::Uranium           => Self::URANIUM,
            Self::Vehicles          => Self::VEHICLES,
            Self::Wood              => Self::WOOD,
            Self::Workers           => Self::WORKERS,
            Self::Yellowcake        => Self::YELLOWCAKE,
        };

        write!(f, "{}", s)
    }
}


impl Display for super::ParticleType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = match self {
            Self::ResidentialHeating => Self::RESIDENTIAL_HEATING,
            Self::BigBlack    => Self::FACTORY_BIG_BLACK,
            Self::MediumBlack => Self::FACTORY_MEDIUM_BLACK,
            Self::SmallBlack  => Self::FACTORY_SMALL_BLACK,
            Self::BigGray     => Self::FACTORY_BIG_GRAY,
            Self::MediumGray  => Self::FACTORY_MEDIUM_GRAY,
            Self::SmallGray   => Self::FACTORY_SMALL_GRAY,
            Self::BigWhite    => Self::FACTORY_BIG_WHITE,
            Self::MediumWhite => Self::FACTORY_MEDIUM_WHITE,
            Self::SmallWhite  => Self::FACTORY_SMALL_WHITE,
            Self::Fountain1   => Self::FOUNTAIN_1,
            Self::Fountain2   => Self::FOUNTAIN_2,
            Self::Fountain3   => Self::FOUNTAIN_3,
        };

        write!(f, "{}", s)
    }
}


impl Display for super::AirplaneStationType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = match self {
            Self::M30 => Self::AIRPLANE_STATION_30M,
            Self::M40 => Self::AIRPLANE_STATION_40M,
            Self::M50 => Self::AIRPLANE_STATION_50M,
            Self::M75 => Self::AIRPLANE_STATION_75M,
        };

        write!(f, "{}", s)
    }
}


impl Display for super::AttractionType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = match self {
            Self::Carousel => Self::ATTRACTION_TYPE_CARUSEL,
            Self::Gallery  => Self::ATTRACTION_TYPE_GALLERY,
            Self::Museum   => Self::ATTRACTION_TYPE_MUSEUM,
            Self::Sight    => Self::ATTRACTION_TYPE_SIGHT,
            Self::Swim     => Self::ATTRACTION_TYPE_SWIM,
            Self::Zoo      => Self::ATTRACTION_TYPE_ZOO,
        };

        write!(f, "{}", s)
    }
}


impl Display for super::ResourceSourceType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = match self {
            Self::Asphalt        => Self::RES_SOURCE_ASPHALT,
            Self::Concrete       => Self::RES_SOURCE_CONCRETE,
            Self::Covered        => Self::RES_SOURCE_COVERED,
            Self::CoveredElectro => Self::RES_SOURCE_COVERED_ELECTRO,
            Self::Gravel         => Self::RES_SOURCE_GRAVEL,
            Self::Open           => Self::RES_SOURCE_OPEN,
            Self::OpenBoards     => Self::RES_SOURCE_OPEN_BOARDS,
            Self::OpenBricks     => Self::RES_SOURCE_OPEN_BRICKS,
            Self::OpenPanels     => Self::RES_SOURCE_OPEN_PANELS,
            Self::Workers        => Self::RES_SOURCE_WORKERS,
        };

        write!(f, "{}", s)
    }
}
