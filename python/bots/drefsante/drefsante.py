############################################################################################################
#                                                                                                          #
# Blood Bowl bot implemented in python but core of the bot is done in Java                                 #
#                                                                                                          #
# This file must be used with interface developed by @njustesen - see https://github.com/njustesen/botbowl #
# See README.txt for set up instruction.                                                                   #
#                                                                                                          #
# This file is not needed when using the Java version UI.                                                  #
#                                                                                                          #
# Development of bot and of its own BloodBowl interface (in Java) are described here:                      #
#                                                                                                          #
#                                  https://drefsante.blogspot.com/                                         #
#                                                                                                          #
# Author: Frederic Bair                                                                                    #
#                                                                                                          #
############################################################################################################
import configparser
import json
import logging
from pathlib import Path

import botbowl
import jnius_config
from botbowl import (
    Action,
    ActionType,
    BBDieResult,
    CasualtyEffect,
    Formation,
    Game,
    OutcomeType,
    ProcBot,
    Square,
    WeatherType,
)
from yasa.components.serializer import GameStateSerializer

config = configparser.ConfigParser()
result = config.read(Path(__file__).with_suffix(".cfg"))
if not result:
    raise FileNotFoundError("Missing config file for drefsante")

jar_path = Path(__file__).parent / config["setup"]["jarPathName"]
if jar_path.exists():
    jnius_config.set_classpath(str(jar_path))
else:
    raise (FileNotFoundError(f"Missing jar file for drefsante at {jar_path}"))

from jnius import JavaClass, JavaMethod, MetaJavaClass, autoclass  # noqa: E402, I001

TEMPORARY_FOLDER = Path(__file__).parent / config["setup"]["temporaryFolder"]
TEMPORARY_FOLDER.mkdir(parents=True, exist_ok=True)

log_dir = Path() / "logs"
log_dir.mkdir(exist_ok=True)

logger = logging.getLogger("drefsante")


class FFAIProxy(JavaClass, metaclass=MetaJavaClass):
    # Note: in case of error at next line, check the jarPathName in drefsante_bot.cfg. The jar must exist.
    __javaclass__ = "be/drefsante/bloodbowl/presenter/FFAIProxy"
    __metaclass__ = MetaJavaClass

    # Signature obtained using command:
    # javap -s C:\Users\drefs\git\bloodbowl-ai\target\classes\be\drefsante\bloodbowl\presenter\FFAIProxy.class

    updateModel = JavaMethod(
        "(IZIZZLjava/lang/String;Ljava/lang/String;ZLjava/lang/String;)V"
    )
    setup = JavaMethod("(ZZ)Ljava/lang/String;")
    playTurn = JavaMethod("(Ljava/lang/String;)Ljava/lang/String;")
    selectPlayerToGetBall = JavaMethod("()Ljava/lang/String;")
    askWhichDice = JavaMethod(
        "(Ljava/util/List;Ljava/lang/String;Ljava/lang/String;Z)I"
    )
    askOnWhichSquarePushed = JavaMethod(
        "(Ljava/lang/String;Ljava/lang/String;Ljava/util/List;Z)I"
    )
    askToFollow = JavaMethod("(Ljava/lang/String;ILjava/lang/String;)Z")
    askUseApothecary = JavaMethod("(Ljava/lang/String;II)Z")
    askPlayerToIntercept = JavaMethod("(Ljava/lang/String;Ljava/util/List;)I")
    selectPlayerForHighKick = JavaMethod("()Ljava/lang/String;")
    selectBallDestinationSquare = JavaMethod("()I")
    askRerollForBlock = JavaMethod(
        "(Ljava/util/List;Ljava/lang/String;Ljava/lang/String;Z)Z"
    )
    askForReroll = JavaMethod("(Ljava/lang/String;)Z")
    askIfPlayerUsesJuggernautSkill = JavaMethod(
        "(Ljava/lang/String;Ljava/lang/String;)Z"
    )
    askIfPlayerUsesWrestleSkill = JavaMethod("(Ljava/lang/String;Ljava/lang/String;)Z")
    askIfPlayerUsesShadowing = JavaMethod("(Ljava/lang/String;Ljava/lang/String;)Z")
    askIfPlayerUsesStandFirmSkill = JavaMethod(
        "(Ljava/lang/String;Ljava/lang/String;)Z"
    )
    askIfPlayerUsesProSkill = JavaMethod("(Ljava/lang/String;)Z")
    askIfPlayerUsesBribe = JavaMethod("(Ljava/lang/String;)Z")
    endGame = JavaMethod("()V")


class DrefsanteBot(ProcBot):
    def __init__(self, name):
        super().__init__(name)
        self.my_team = None
        self.opp_team = None
        self.actions = []
        self.last_turn = 0
        self.last_half = 0
        self.setup_actions = []
        self.ffaiProxy = None
        self.oldDefensorPosition = -1
        self.name = name

        # !!! !!!
        # Formations used only when there is a heated player
        # while issue https://github.com/njustesen/botbowl/issues/240 is not fixed
        self.off_formation = [
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "m", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "x", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "S"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "x"],
            ["-", "-", "-", "-", "-", "s", "-", "-", "-", "0", "-", "-", "S"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "x"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "S"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "x", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "m", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
        ]

        self.def_formation = [
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "x", "-", "b", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "x", "-", "S", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "0"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "0"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "0"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "x", "-", "S", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "x", "-", "b", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
            ["-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-"],
        ]

        self.off_formation = Formation("Line offense", self.off_formation)
        self.def_formation = Formation("Spread defense", self.def_formation)

    def new_game(self, game, team):
        """
        Called when a new game starts.
        """
        self.my_team = team
        self.opp_team = game.get_opp_team(team)
        self.last_turn = 0
        self.last_half = 0

        FFAIProxy = autoclass("be.drefsante.bloodbowl.presenter.FFAIProxy")
        self.ffaiProxy = FFAIProxy()

        self.temporaryFolder = TEMPORARY_FOLDER

    def act(self, game: Game) -> Action:
        log_file = log_dir / f"game_{game.game_id}.jsonl"
        game_state_json = GameStateSerializer.to_json(game.state)
        with open(log_file, "a") as f:
            json.dump(game_state_json, f)
            f.write("\n")

        return super().act(game)

    def coin_toss_flip(self, game):
        return Action(ActionType.TAILS)

    def coin_toss_kick_receive(self, game):
        return Action(ActionType.RECEIVE)

    def convertTeam(self, game, teamFFAI):
        # ArrayList = autoclass('java.util.ArrayList')
        team = []  # ArrayList()

        self.convertedListOfPlayers(game, teamFFAI.players, team)
        team.append(teamFFAI.race)

        teamName = teamFFAI.name + "_away"
        if teamFFAI == game.state.home_team:
            teamName = teamFFAI.name + "_home"
        team.append(teamName)
        team.append(teamFFAI.rerolls)  # initial
        team.append(teamFFAI.cheerleaders)
        team.append(teamFFAI.fan_factor)
        team.append(teamFFAI.ass_coaches)
        team.append(teamFFAI.apothecaries > 0)
        team.append(teamFFAI.apothecaries == 0)  # TODO to check <- False
        team.append(teamFFAI.state.rerolls)  # remaining
        team.append(not game.is_foul_available())
        team.append(not game.is_blitz_available())
        team.append(not game.is_handoff_available())
        team.append(not game.is_pass_available())
        team.append(teamFFAI.state.reroll_used)
        team.append(teamFFAI.state.turn)
        team.append(game.is_home_team(teamFFAI))
        team.append(teamFFAI.state.score)
        team.append(teamFFAI.state.bribes)

        # team.append(len(tempArray))
        # for value in tempArray:
        #    team.append(value)

        return team

    def getProxyPosition(self, position):
        if position is None:
            return Square(-1, -1)
        return position

    def convertedListOfPlayers(self, game, units, players):
        # ArrayList = autoclass('java.util.ArrayList')
        if players is None:
            players = []  # ArrayList()

        if units is None:
            players.append(0)
            return players

        tempArray = []  # ArrayList()

        for unit in units:
            self.convertPlayer(game, unit, tempArray)

        players.append(len(tempArray) + 1)  # num of elements
        players.append(len(units))  # num of players
        for value in tempArray:
            players.append(value)

        return players

    def convertPlayer(self, game, unit, player):
        # ArrayList = autoclass('java.util.ArrayList')
        if player is None:
            player = []  # ArrayList()

        if unit is None:
            player.append(0)
            return player

        tempArray = []  # ArrayList()

        tempArray.append(self.getName(game, unit))
        tempArray.append(unit.role.name)
        tempArray.append(unit.role.cost)
        tempArray.append(unit.role.ma)
        tempArray.append(unit.role.st)
        tempArray.append(unit.role.ag)
        tempArray.append(unit.role.av)
        tempArray.append(unit in game.get_players_on_pitch(unit.team))

        tempArray.append(self.getSquareId(unit.position))

        tempArray.append(unit in game.get_reserves(unit.team))
        tempArray.append(unit in game.get_knocked_out(unit.team))

        isDead = CasualtyEffect.DEAD in unit.state.injuries_gained

        tempArray.append(isDead)
        tempArray.append(not isDead and len(unit.state.injuries_gained) > 0)  # injuried

        tempArray.append(
            unit in game.get_dungeon(unit.team)
        )  # TODO to check # excluded

        hasActionType = (
            unit == game.get_active_player()
            and game.get_player_action_type() is not None
        )
        tempArray.append(hasActionType)

        if hasActionType:
            tempArray.append(game.get_player_action_type().value)
        else:
            tempArray.append(None)

        tempArray.append(unit.state.used)

        tempArray.append(unit.state.hypnotized)
        tempArray.append(unit.state.bone_headed)
        tempArray.append(unit.state.really_stupid)
        tempArray.append(unit.state.taken_root)
        tempArray.append(unit.state.wild_animal)  # todo check
        tempArray.append(unit.state.has_blocked)
        tempArray.append(unit.state.stunned)
        tempArray.append(len(unit.state.injuries_gained) > 0)
        tempArray.append(unit.state.knocked_out)

        tempArray.append(unit.state.heated)
        tempArray.append(not unit.state.up)

        if (
            not unit.state.up
            and game.get_ball_position() is not None
            and game.get_ball_position() == unit.position
        ):
            logger.debug("Knocked out player has the ball!!!")

        tempArray.append(unit.num_moves_left(include_gfi=True))
        tempArray.append(unit.num_moves_left(include_gfi=False))
        # tempArray.append(unit.player_id)
        self.convertSkills(unit, tempArray)

        player.append(len(tempArray))
        for value in tempArray:
            player.append(value)

        return player
        # return ListConverter().convert(player, self.gateway._gateway_client)

    def convertSkills(self, unit, skills):
        # ArrayList = autoclass('java.util.ArrayList')
        if unit is None:
            skills.append(0)
            return skills

        tempArray = []  # ArrayList()

        for skill in unit.role.skills:
            tempArray.append(skill.value)

        skills.append(len(tempArray))
        for value in tempArray:
            skills.append(value)

        return skills

    def writeTeam(self, fileName, team):
        with open(fileName, "w+") as f:
            team = map(lambda x: str(x) + "\n", team)
            f.writelines(team)

    def updateModel(self, game):
        filePrefixWithPath = self.temporaryFolder / str(game.game_id)
        filePrefixWithPath.mkdir(parents=True, exist_ok=True)

        # Using file to update data to Java seems faster than to use pyjnius
        # However updating full model is really slow, and it should be improved to
        # update only player status, position, ball, etc...
        # Currently, this is the CPU bottleneck
        self.writeTeam(
            filePrefixWithPath / "team1", self.convertTeam(game, self.my_team)
        )
        self.writeTeam(
            filePrefixWithPath / "team2", self.convertTeam(game, self.opp_team)
        )

        # Boolean = autoclass('java.lang.Boolean')
        isBlitz = False  # Boolean(False)
        if game.is_blitz():
            isBlitz = game.is_blitz()
        isQuickSnap = False  # Boolean(False)
        if game.is_quick_snap():
            isQuickSnap = game.is_quick_snap()

        isActivePlayerHasAction = game.get_player_action_type() is not None
        isBallInAir = False
        # isBlitz = Boolean(False)
        if (
            game.get_ball_position() is not None
            and game.get_ball_at(game.get_ball_position()) is None
        ):
            isBallInAir = True

        self.ffaiProxy.updateModel(
            self.getSquareId(game.get_ball_position()),
            isBallInAir,
            game.state.half,
            isBlitz,
            isQuickSnap,
            self.convertWeather(game.get_weather()),
            # self.convertTeam(game, self.my_team), \
            # self.convertTeam(game, self.opp_team), \
            self.getName(game, game.get_ball_carrier()),
            isActivePlayerHasAction,
            str(filePrefixWithPath) + "/",
        )  # , \
        # isBlitz) # TODO change to isBlitz action ???

        return

    def convertWeather(self, weather):
        if weather == WeatherType.SWELTERING_HEAT:
            return "HEAT"
        if weather == WeatherType.VERY_SUNNY:
            return "SUNNY"
        if weather == WeatherType.NICE:
            return "GOOD"
        if weather == WeatherType.POURING_RAIN:
            return "RAIN"
        if weather == WeatherType.BLIZZARD:
            return "BLIZZARD"

        return None

    def getUnitFromName(self, game, name):
        for unit in self.my_team.players:
            if self.getName(game, unit) == name:
                return unit

        for unit in self.opp_team.players:
            if self.getName(game, unit) == name:
                return unit

        return None

    def getName(self, game, unit):
        if unit is None:
            return ""

        if unit.team == game.state.home_team:
            return unit.name + "_home"
        return unit.name + "_away"

    def setup(self, game):
        # Update teams
        self.my_team = game.get_team_by_id(self.my_team.team_id)
        self.opp_team = game.get_opp_team(self.my_team)

        if self.setup_actions:
            action = self.setup_actions.pop(0)
            logger.debug(
                "Executing action ", action.action_type, "; Player = ", action.player
            )
            return action

        # Due to bug https://github.com/njustesen/botbowl/issues/240
        # we select pre-defined formation if player with status heat is detected
        for unit in self.my_team.players:
            if unit.state.heated:
                return self.setupIfHeatedDetected(game)

        self.updateModel(game)

        isReceiving = game.get_receiving_team() == self.my_team
        playersAndPositions = self.ffaiProxy.setup(
            isReceiving, game.get_procedure().reorganize
        )

        if playersAndPositions == "":  # happen when reorganize
            self.setup_actions.append(Action(ActionType.END_SETUP))
            return

        actions = playersAndPositions.split("\n")
        for action in actions:
            keys = action.split(";")
            playerName = keys[0]

            if len(keys) < 2:
                self.setup_actions.append(Action(ActionType.END_SETUP))

            position = self.getPosition(int(keys[1]))
            self.setup_actions.append(
                Action(
                    ActionType.PLACE_PLAYER,
                    position=position,
                    player=self.getUnitFromName(game, playerName),
                )
            )

        self.setup_actions.append(Action(ActionType.END_SETUP))
        action = self.setup_actions.pop(0)
        return action

    def setupIfHeatedDetected(self, game):
        """
        Set up if one player has status heated
        """
        if game.get_receiving_team() == self.my_team:
            self.setup_actions = self.off_formation.actions(game, self.my_team)
            self.setup_actions.append(Action(ActionType.END_SETUP))
        else:
            self.setup_actions = self.def_formation.actions(game, self.my_team)
            self.setup_actions.append(Action(ActionType.END_SETUP))
        action = self.setup_actions.pop(0)
        return action

    def getBBDices(self, dicesFFAI):
        ArrayList = autoclass("java.util.ArrayList")
        dices = ArrayList()
        for dice in dicesFFAI:
            dices.add(dice.get_value().value)
        return dices

    def getDices(self, actions):
        ArrayList = autoclass("java.util.ArrayList")
        dices = ArrayList()
        for action in actions:
            if action == ActionType.SELECT_ATTACKER_DOWN:
                dices.add(BBDieResult.ATTACKER_DOWN.value)
            elif action == ActionType.SELECT_BOTH_DOWN:
                dices.add(BBDieResult.BOTH_DOWN.value)
            elif action == ActionType.SELECT_PUSH:
                dices.add(BBDieResult.PUSH.value)
            elif action == ActionType.SELECT_DEFENDER_STUMBLES:
                dices.add(BBDieResult.DEFENDER_STUMBLES.value)
            elif action == ActionType.SELECT_DEFENDER_DOWN:
                dices.add(BBDieResult.DEFENDER_DOWN.value)

        return dices

    def reroll(self, game):
        self.updateModel(game)

        """
        Select between USE_REROLL and DONT_USE_REROLL
        """
        reroll_proc = game.get_procedure()
        context = reroll_proc.context

        if isinstance(context, botbowl.Block):
            attacker = context.attacker
            defender = context.defender
            dices = self.getBBDices(context.roll.dice)

            isTeamWillSelectDice = (
                context.favor == self.my_team
            )  # self.gateway.jvm.be.drefsante.bloodbowl.presenter.Lrb6Rules.isTeamWillSelectDice(self.getPlayer(game, attacker),

            if not isTeamWillSelectDice:
                logger.debug("Team will NOT Select dice ")

            isReroll = self.ffaiProxy.askRerollForBlock(
                dices,
                self.getName(game, attacker),
                self.getName(game, defender),
                isTeamWillSelectDice,
            )

            if isReroll:
                logger.debug("Using reroll because bad dices for block")
                return Action(ActionType.USE_REROLL)

            return Action(ActionType.DONT_USE_REROLL)

        unit = reroll_proc.player
        isReroll = self.ffaiProxy.askForReroll(self.getName(game, unit))
        if isReroll:
            return Action(ActionType.USE_REROLL)
        return Action(ActionType.DONT_USE_REROLL)

    def place_ball(self, game):
        """
        Place the ball when kicking.
        """
        self.updateModel(game)

        id = self.ffaiProxy.selectBallDestinationSquare()

        return Action(ActionType.PLACE_BALL, position=self.getPosition(id))

    def high_kick(self, game):
        """
        Select player to move under the ball.
        """
        self.updateModel(game)

        playerName = self.ffaiProxy.selectPlayerForHighKick()

        if playerName != "":
            return Action(
                ActionType.SELECT_PLAYER,
                player=self.getUnitFromName(game, playerName),
                position=game.get_ball_position(),
            )

        return Action(ActionType.SELECT_NONE)

    def touchback(self, game):
        """
        Select player to give the ball to.
        """
        self.updateModel(game)

        playerName = self.ffaiProxy.selectPlayerToGetBall()

        return Action(
            ActionType.SELECT_PLAYER, player=self.getUnitFromName(game, playerName)
        )

    def turn(self, game):
        # Update teams
        self.my_team = game.get_team_by_id(self.my_team.team_id)
        self.opp_team = game.get_opp_team(self.my_team)

        """
        Start a new player action.
        """

        # Reset actions if new turn
        turn = game.get_agent_team(self).state.turn
        half = game.state.half
        if half > self.last_half or turn > self.last_turn:
            self.actions.clear()
            self.last_turn = turn
            self.last_half = half
            self.actions = []

        # End turn if only action left
        if len(game.state.available_actions) == 1:
            if game.state.available_actions[0].action_type == ActionType.END_TURN:
                self.actions = [Action(ActionType.END_TURN)]

        # Execute planned actions if any
        if len(self.actions) > 0:
            action = self.protect(game, self._get_next_action())
            if action is not None:
                return action
            else:
                # Planned action is None: can occur for example if 'Wild Animal' has failed
                # -> then we remove all planned action and make another plan
                logger.debug("Planned action is not possible!!!")
                self.actions.clear()

        # Split logic depending on offense, defense, and loose ball - and plan actions
        self._make_plan(game)
        action = self.protect(game, self._get_next_action())
        return action

    def protect(self, game, action):
        if self.isOptionAvailable(game, action.action_type):
            return action

        # if action.action_type == ActionType.HANDOFF or action.action_type == ActionType.FOUL:
        #    # This change is needed if PAthFinding is enbabled - see https://github.com/njustesen/ffai/issues/162
        #    return self.actions.append(Action(ActionType.MOVE, position=action.position, player=action.player))

        activePlayerName = self.getActivePlayerName(game)
        if activePlayerName == "":
            return None
            # return self.actions.append(Action(ActionType.END_TURN))
        return None
        # return self.actions.append(Action(ActionType.END_PLAYER_TURN, player=game.get_active_player()))

    def _get_next_action(self):
        action = self.actions[0]
        self.actions = self.actions[1:]

        return action

    def isOptionAvailable(self, game, actionType):
        for action in game.get_available_actions():
            if action.action_type == actionType:
                return True
        return False

    def isStartOptionAvailable(self, game):
        return self.isOptionAvailable(game, ActionType.START_MOVE)

    def isUndoOptionAvailable(self, game):
        return self.isOptionAvailable(game, ActionType.UNDO)

    def getActivePlayerName(self, game):
        if game.get_active_player() is not None:
            if self.isStartOptionAvailable(game):
                return ""
            return self.getName(game, game.get_active_player())
        return ""

    def _make_plan(self, game):
        activeplayerName = self.getActivePlayerName(game)
        if activeplayerName != "" and self.isUndoOptionAvailable(game):
            logger.debug("bug - to investigate")
            self.actions.append(
                Action(ActionType.END_PLAYER_TURN, player=game.get_active_player())
            )
            return

        self.updateModel(game)
        actionAI = self.ffaiProxy.playTurn(activeplayerName)

        logger.debug(actionAI)
        actions = actionAI.split("\n")
        for action in actions:
            keys = action.split(";")
            actionKey = keys[0]

            if actionKey == "START_MOVE":
                mover = self.getUnitFromName(game, keys[1])
                self.actions.append(Action(ActionType.START_MOVE, player=mover))
            elif actionKey == "START_BLITZ":
                mover = self.getUnitFromName(game, keys[1])
                self.actions.append(Action(ActionType.START_BLITZ, player=mover))
            elif actionKey == "START_BLOCK":
                mover = self.getUnitFromName(game, keys[1])
                self.actions.append(Action(ActionType.START_BLOCK, player=mover))
            elif actionKey == "START_PASS":
                mover = self.getUnitFromName(game, keys[1])
                self.actions.append(Action(ActionType.START_PASS, player=mover))
            elif actionKey == "START_HANDOFF":
                mover = self.getUnitFromName(game, keys[1])
                self.actions.append(Action(ActionType.START_HANDOFF, player=mover))
            elif actionKey == "START_FOUL":
                mover = self.getUnitFromName(game, keys[1])
                self.actions.append(Action(ActionType.START_FOUL, player=mover))
            elif actionKey == "MOVE":
                position = self.getPosition(int(keys[1]))
                mover = self.getUnitFromName(game, keys[2])
                self.actions.append(Action(ActionType.MOVE, position, mover))
            elif actionKey == "END_PLAYER_TURN":
                mover = self.getUnitFromName(game, keys[1])
                self.actions.append(Action(ActionType.END_PLAYER_TURN, player=mover))
            elif actionKey == "STAND_UP":
                mover = self.getUnitFromName(game, keys[1])
                if mover.state.up:
                    logger.debug("Try to rise up player not prone")
                self.actions.append(Action(ActionType.STAND_UP, player=mover))
            elif actionKey == "BLOCK":
                position = self.getPosition(int(keys[1]))
                mover = self.getUnitFromName(game, keys[2])
                self.actions.append(Action(ActionType.BLOCK, position, mover))
            elif actionKey == "PASS":
                position = self.getPosition(int(keys[1]))
                mover = self.getUnitFromName(game, keys[2])
                self.actions.append(Action(ActionType.PASS, position, mover))
            elif actionKey == "HANDOFF":
                position = self.getPosition(int(keys[1]))
                mover = self.getUnitFromName(game, keys[2])
                self.actions.append(Action(ActionType.HANDOFF, position, mover))
            elif actionKey == "PICKUP_TEAM_MATE":
                mover = self.getUnitFromName(game, keys[1])
                self.actions.append(
                    Action(ActionType.PICKUP_TEAM_MATE, mover)
                )  # TODO check
            elif actionKey == "THROW_TEAM_MATE":
                position = self.getPosition(int(keys[1]))
                mover = self.getUnitFromName(game, keys[2])
                self.actions.append(
                    Action(ActionType.THROW_TEAM_MATE, position, mover)
                )  # TODO check
            elif actionKey == "LEAP":
                position = self.getPosition(int(keys[1]))
                mover = self.getUnitFromName(game, keys[2])
                self.actions.append(
                    Action(ActionType.LEAP, position, mover)
                )  # TODO check
            elif actionKey == "END_TURN":
                self.actions.append(Action(ActionType.END_TURN))
            elif actionKey == "FOUL":
                position = self.getPosition(int(keys[1]))
                mover = self.getUnitFromName(game, keys[2])
                self.actions.append(Action(ActionType.FOUL, position, mover))

        return

    def quick_snap(self, game):
        self.actions.clear()  # for example if we score, last action (END_PLAYER_TURN) is not yet executed -> we have to remove it

        return self.turn(game)

    def blitz(self, game):
        self.actions.clear()  # for example if we score, last action (END_PLAYER_TURN) is not yet executed -> we have to remove it

        return self.turn(game)

    def player_action(self, game):
        # Execute planned actions if any
        if len(self.actions) > 0:
            action = self.protect(game, self._get_next_action())
            return action

        logger.debug("player action !!!!")

        if game.get_active_player().state.has_blocked:
            logger.debug(
                game.get_active_player().name, " has blitzed and finish to play"
            )
            # return self.turn(game)
            # return Action(ActionType.END_PLAYER_TURN, player=game.get_active_player())
        return self.turn(game)

    def block(self, game):
        """
        Select block die or reroll.
        """
        self.updateModel(game)

        attacker = game.get_procedure().attacker
        defender = game.get_procedure().defender
        is_blitz = game.get_procedure().blitz

        isRerollAvailable = False
        actions = []
        for action_choice in game.state.available_actions:
            if action_choice.action_type == ActionType.USE_REROLL:
                isRerollAvailable = True
            else:
                actions.append(action_choice.action_type)

        dices = self.getDices(actions)

        if isRerollAvailable:
            # 01-Sep-2021 Looks it will never happen ????
            # isReroll = self.presenter.getSelectedTeam().getCoachIA().getDialoger().askRerollForBlock(dices, self.getPlayer(game,attacker), self.getPlayer(game,defender), isTeamWillSelectDice)

            logger.debug("check why we are here! Should not be possible!!!")
            return Action(ActionType.USE_REROLL)

        result = self.ffaiProxy.askWhichDice(
            dices, self.getName(game, attacker), self.getName(game, defender), is_blitz
        )  # , selectedTeam)

        action = Action(actions[result])

        if action is None:
            logger.debug("Action is none in proc block !!!")

        return action

    def getPosition(self, squareId):
        i = squareId // 26 + 1
        j = squareId % 26 + 1

        return Square(j, i)

    def getSquareId(self, position):
        if position is None:
            return -1

        j = position.x - 1
        i = position.y - 1

        if i >= 0 and i < 15 and j >= 0 and j < 26:
            return i * 26 + j

        return -1

    def getSquares(self, positions):
        ArrayList = autoclass("java.util.ArrayList")
        finalSquares = ArrayList()

        for position in positions:
            finalSquares.add(self.getSquareId(position))

        return finalSquares

    def push(self, game):
        """
        Select square to push to.
        """
        self.updateModel(game)

        pushedPlayer = game.get_procedure().player
        pusherPlayer = game.get_procedure().pusher
        authorizedSquares = self.getSquares(game.state.available_actions[0].positions)

        if not game.get_procedure().chain:
            self.oldDefensorPosition = self.getSquareId(pushedPlayer.position)

        isFirstPush = not game.get_procedure().chain

        result = self.ffaiProxy.askOnWhichSquarePushed(
            self.getName(game, pusherPlayer),
            self.getName(game, pushedPlayer),
            authorizedSquares,
            isFirstPush,
        )
        resultSquareId = authorizedSquares.get(result)

        for position in game.state.available_actions[0].positions:
            if self.getSquareId(position) == resultSquareId:
                logger.debug(
                    "Position selected = ", position, "; SquareId = ", resultSquareId
                )
                return Action(ActionType.PUSH, position=position)

        logger.debug("none square to push???")

    def follow_up(self, game):
        """
        Follow up or not. ActionType.FOLLOW_UP must be used together with a position.
        """
        self.updateModel(game)
        mover = game.state.active_player
        defender = game.get_procedure().defender

        result = self.ffaiProxy.askToFollow(
            self.getName(game, mover),
            self.oldDefensorPosition,
            self.getName(game, defender),
        )

        if result:
            return Action(ActionType.FOLLOW_UP, position=game.get_procedure().pos_to)

        return Action(ActionType.FOLLOW_UP, position=game.state.active_player.position)

    def apothecary(self, game):
        """
        Use apothecary?
        """
        self.updateModel(game)
        isUseApo = False

        # TODO test now for KO

        if game.get_procedure().outcome == OutcomeType.KNOCKED_OUT:
            # In fact no need to call self.ffaiProxy.askUseApothecaryForKO, this is always false
            isUseApo = False
        else:
            isUseApo = self.ffaiProxy.askUseApothecary(
                self.getName(game, game.get_procedure().player),
                game.get_procedure().roll_first,
                game.get_procedure().roll_second,
            )

        if isUseApo:
            return Action(ActionType.USE_APOTHECARY)
        return Action(ActionType.DONT_USE_APOTHECARY)

    def interception(self, game):
        """
        Select interceptor.
        """
        if len(game.get_procedure().interceptors) == 0:
            return Action(ActionType.SELECT_NONE)

        self.updateModel(game)

        ArrayList = autoclass("java.util.ArrayList")
        interceptorNames = ArrayList()
        for interceptor in game.get_procedure().interceptors:
            interceptorNames.add(self.getName(game, interceptor))

        index = self.ffaiProxy.askPlayerToIntercept(
            self.getName(game, game.get_procedure().passer), interceptorNames
        )

        return Action(
            ActionType.SELECT_PLAYER, player=game.get_procedure().interceptors[index]
        )

    def pass_action(self, game):
        return self.askForReroll(game, game.get_procedure().passer)

    def catch(self, game):
        return self.askForReroll(game, game.get_procedure().player)

    def gfi(self, game):
        return self.askForReroll(game, game.get_procedure().player)

    def dodge(self, game):
        return self.askForReroll(game, game.get_procedure().player)

    def pickup(self, game):
        return self.askForReroll(game, game.get_procedure().player)

    def askForReroll(self, game, unit):
        self.updateModel(game)
        isUseReroll = self.ffaiProxy.askForReroll(self.getName(game, unit))

        if isUseReroll:
            return Action(ActionType.USE_REROLL)
        return Action(ActionType.DONT_USE_REROLL)

    def use_juggernaut(self, game):
        self.updateModel(game)
        isUseSkill = self.ffaiProxy.askIfPlayerUsesJuggernautSkill(
            self.getName(game, game.get_procedure().attacker),
            self.getName(game, game.get_procedure().defender),
        )

        if isUseSkill:
            return Action(ActionType.USE_SKILL)
        return Action(ActionType.DONT_USE_SKILL)

    def use_wrestle(self, game):
        self.updateModel(game)
        isUseSkill = self.ffaiProxy.askIfPlayerUsesWrestleSkill(
            self.getName(game, game.get_procedure().attacker),
            self.getName(game, game.get_procedure().defender),
        )

        if isUseSkill:
            return Action(ActionType.USE_SKILL)
        return Action(ActionType.DONT_USE_SKILL)

    def use_shadowing(self, game):
        self.updateModel(game)
        isUseSkill = self.ffaiProxy.askIfPlayerUsesShadowing(
            self.getName(game, game.get_procedure().shadower),
            self.getName(game, game.get_procedure().player),
        )

        if isUseSkill:
            return Action(ActionType.USE_SKILL)
        return Action(ActionType.DONT_USE_SKILL)

    def use_stand_firm(self, game):
        self.updateModel(game)
        isUseSkill = self.ffaiProxy.askIfPlayerUsesStandFirmSkill(
            self.getName(game, game.get_procedure().pusher),
            self.getName(game, game.get_procedure().player),
        )

        if isUseSkill:
            return Action(ActionType.USE_SKILL)
        return Action(ActionType.DONT_USE_SKILL)

    def use_pro(self, game):
        self.updateModel(game)
        isUseSkill = self.ffaiProxy.askIfPlayerUsesProSkill(
            self.getName(game, game.get_procedure().player)
        )

        if isUseSkill:
            return Action(ActionType.USE_SKILL)
        return Action(ActionType.DONT_USE_SKILL)

    def use_bribe(self, game):
        self.updateModel(game)
        isUseBribe = self.ffaiProxy.askIfPlayerUsesBribe(
            self.getName(game, game.get_procedure().player)
        )

        if isUseBribe:
            return Action(ActionType.USE_BRIBE)
        return Action(ActionType.DONT_USE_BRIBE)

    def blood_lust_block_or_move(self, game):
        # TODO to implement
        return Action(ActionType.START_BLOCK)

    def eat_thrall(self, game):
        # TODO to implement
        position = game.get_available_actions()[0].positions[0]
        return Action(ActionType.SELECT_PLAYER, position)

    def perfect_defense(self, game):
        # TODO to implement

        return Action(ActionType.END_SETUP)

    def handle_illegal_action(self, game, action):
        """
        Called when there is an error - should not occur normally, always investigate when here
        """
        logger.debug(f"{action} is not allowed. ")
        return game._forced_action()

    def end_game(self, game: Game):
        """
        Called when a game end.
        """
        self.ffaiProxy.endGame()

        filePrefixWithPath = Path(self.temporaryFolder) / game.game_id

        if (filePrefixWithPath / "team1").exists():
            (filePrefixWithPath / "team1").unlink()
        if (filePrefixWithPath / "team2").exists():
            (filePrefixWithPath / "team2").unlink()
